use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

mod registers;
mod mmu;
mod timer;
mod input;
mod ppu;
mod apu;

use registers::Registers;
use mmu::MMU;
use timer::Timer;
use input::Input;
use ppu::PPU;
use apu::APU;

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

use std::cell::RefCell;

thread_local! {
    static GB_SINGLETON: RefCell<Option<GameBoy>> = RefCell::new(None);
}

#[wasm_bindgen]
pub struct GameBoy {
    running: bool,
    mmu: MMU,
    registers: Registers,
    timer: Timer,
    input: Input,
    ppu: PPU,
    apu: APU,
    cycles: u32,
    halted: bool,
    ime: bool,
    ime_scheduled: bool,
    // Debug trace of last N opcodes
    trace_enabled: bool,
    trace_buf: [(u16, u8, u16); 256],
    trace_idx: usize,
    last_interrupt: Option<(u8, u16, u8, u8)>, // (interrupt id, pc before jump, IE, IF)
}

#[derive(Serialize, Deserialize)]
struct SaveState {
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
    cycles: u32,
}

#[wasm_bindgen]
impl GameBoy {
    #[wasm_bindgen(constructor)]
    pub fn new() -> GameBoy {
        GameBoy {
            running: false,
            mmu: MMU::new(),
            registers: Registers::new(),
            timer: Timer::new(),
            input: Input::new(),
            ppu: PPU::new(),
            apu: APU::new(),
            cycles: 0,
            halted: false,
            ime: false,
            ime_scheduled: false,
            trace_enabled: false,
            trace_buf: [(0, 0, 0); 256],
            trace_idx: 0,
            last_interrupt: None,
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.mmu.load_rom(rom);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.running = false;
        self.mmu.reset();
        self.registers = Registers::new();
        self.timer.reset();
        self.input.reset();
        self.ppu.reset(&mut self.mmu);
        self.apu.reset();
        self.cycles = 0;
        self.halted = false;
        self.ime = false;
        self.ime_scheduled = false;
        self.trace_idx = 0;
        self.trace_buf.fill((0, 0, 0));
        self.last_interrupt = None;
    }

    pub fn start(&mut self) { self.running = true; }
    pub fn stop(&mut self) { self.running = false; }
    pub fn is_running(&self) -> bool { self.running }

    pub fn run_frame(&mut self) -> bool {
        if !self.running { return false; }

        let target_cycles = 70224;
        let mut frame_cycles = 0;
        let mut frame_ready = false;

        while frame_cycles < target_cycles {
            let cpu_cycles = self.step_cpu();
            frame_cycles += cpu_cycles;
            
            // Update peripherals
            self.timer.step(cpu_cycles, self.mmu.get_io_mut());
            self.apu.step(&self.mmu, cpu_cycles);
            
            // PPU returns true when a frame is ready
            if self.ppu.step(&mut self.mmu, cpu_cycles) {
                frame_ready = true;
            }
        }

        frame_ready
    }

    fn step_cpu(&mut self) -> u32 {
        if self.halted {
            // Check for pending interrupts even when halted
            if self.check_interrupts().is_some() {
                self.halted = false;
            }
            return 4;
        }

        let cycles_before = self.cycles;

        if self.ime_scheduled {
            self.ime = true;
            self.ime_scheduled = false;
        }

        // Only service interrupts if IME is enabled
        if self.ime {
            if let Some(interrupt) = self.check_interrupts() {
                self.handle_interrupt(interrupt);
                return self.cycles - cycles_before;
            }
        }

        let pc_before = self.registers.pc;
        let opcode = self.fetch_byte();
        if self.trace_enabled {
            self.trace_buf[self.trace_idx & 0xff] = (pc_before, opcode, self.registers.sp);
            self.trace_idx = self.trace_idx.wrapping_add(1);
        }
        self.execute_opcode(opcode);

        self.cycles - cycles_before
    }

    fn check_interrupts(&self) -> Option<u8> {
        let ie = self.mmu.read_byte(0xffff);
        let if_ = self.mmu.read_byte(0xff0f);
        let interrupts = ie & if_;
        if interrupts == 0 { return None; }
        for i in 0..5 {
            if interrupts & (1 << i) != 0 {
                return Some(i);
            }
        }
        None
    }

    fn handle_interrupt(&mut self, interrupt: u8) {
        self.ime = false;
        self.halted = false;
        let if_ = self.mmu.read_byte(0xff0f);
        self.mmu.write_byte(0xff0f, if_ & !(1 << interrupt));
        let pc_before = self.registers.pc;
        let ie = self.mmu.read_byte(0xffff);
        self.last_interrupt = Some((interrupt, pc_before, ie, if_));
        
        // Guard against stack overflow during rapid interrupt loops
        if self.registers.sp < 0x8100 {
            // Stack has grown dangerously large; likely stuck in interrupt loop
            // Don't service this interrupt; let the ROM recover
            self.mmu.write_byte(0xff0f, 0); // Clear all IF flags
            return;
        }
        
        self.push_word(self.registers.pc);
        let handlers = [0x40, 0x48, 0x50, 0x58, 0x60];
        self.registers.pc = handlers[interrupt as usize];
        self.cycles += 20;
    }    fn fetch_byte(&mut self) -> u8 {
        let byte = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        byte
    }

    fn fetch_word(&mut self) -> u16 {
        let lo = self.fetch_byte() as u16;
        let hi = self.fetch_byte() as u16;
        (hi << 8) | lo
    }

    fn push_word(&mut self, val: u16) {
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.mmu.write_byte(self.registers.sp, (val >> 8) as u8);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.mmu.write_byte(self.registers.sp, val as u8);
    }

    fn pop_word(&mut self) -> u16 {
        let lo = self.mmu.read_byte(self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let hi = self.mmu.read_byte(self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        (hi << 8) | lo
    }

    fn execute_opcode(&mut self, opcode: u8) {
        match opcode {
            // 0x00: NOP
            0x00 => self.cycles += 4,
            
            // 0x01: LD BC, nn
            0x01 => {
                let val = self.fetch_word();
                self.registers.set_bc(val);
                self.cycles += 12;
            }
            
            // 0x02: LD (BC), A
            0x02 => {
                self.mmu.write_byte(self.registers.bc(), self.registers.a);
                self.cycles += 8;
            }
            
            // 0x03: INC BC
            0x03 => {
                let val = self.registers.bc().wrapping_add(1);
                self.registers.set_bc(val);
                self.cycles += 8;
            }
            
            // 0x04: INC B
            0x04 => {
                self.registers.b = self.inc8(self.registers.b);
                self.cycles += 4;
            }
            
            // 0x05: DEC B
            0x05 => {
                self.registers.b = self.dec8(self.registers.b);
                self.cycles += 4;
            }
            
            // 0x06: LD B, n
            0x06 => {
                self.registers.b = self.fetch_byte();
                self.cycles += 8;
            }
            
            // 0x07: RLCA
            0x07 => {
                self.rlca();
                self.cycles += 4;
            }
            
            // 0x08: LD (nn), SP
            0x08 => {
                let addr = self.fetch_word();
                self.mmu.write_byte(addr, (self.registers.sp & 0xff) as u8);
                self.mmu.write_byte(addr.wrapping_add(1), ((self.registers.sp >> 8) & 0xff) as u8);
                self.cycles += 20;
            }
            
            // 0x09: ADD HL, BC
            0x09 => {
                self.add_hl(self.registers.bc());
                self.cycles += 8;
            }
            
            // 0x0A: LD A, (BC)
            0x0a => {
                self.registers.a = self.mmu.read_byte(self.registers.bc());
                self.cycles += 8;
            }
            
            // 0x0B: DEC BC
            0x0b => {
                let val = self.registers.bc().wrapping_sub(1);
                self.registers.set_bc(val);
                self.cycles += 8;
            }
            
            // 0x0C: INC C
            0x0c => {
                self.registers.c = self.inc8(self.registers.c);
                self.cycles += 4;
            }
            
            // 0x0D: DEC C
            0x0d => {
                self.registers.c = self.dec8(self.registers.c);
                self.cycles += 4;
            }
            
            // 0x0E: LD C, n
            0x0e => {
                self.registers.c = self.fetch_byte();
                self.cycles += 8;
            }
            
            // 0x0F: RRCA
            0x0f => {
                self.rrca();
                self.cycles += 4;
            }
            
            // 0x10: STOP
            0x10 => {
                self.fetch_byte(); // STOP takes 2 bytes
                self.cycles += 4;
            }
            
            // 0x11: LD DE, nn
            0x11 => {
                let val = self.fetch_word();
                self.registers.set_de(val);
                self.cycles += 12;
            }
            
            // 0x12: LD (DE), A
            0x12 => {
                self.mmu.write_byte(self.registers.de(), self.registers.a);
                self.cycles += 8;
            }
            
            // 0x13: INC DE
            0x13 => {
                let val = self.registers.de().wrapping_add(1);
                self.registers.set_de(val);
                self.cycles += 8;
            }
            
            // 0x14: INC D
            0x14 => {
                self.registers.d = self.inc8(self.registers.d);
                self.cycles += 4;
            }
            
            // 0x15: DEC D
            0x15 => {
                self.registers.d = self.dec8(self.registers.d);
                self.cycles += 4;
            }
            
            // 0x16: LD D, n
            0x16 => {
                self.registers.d = self.fetch_byte();
                self.cycles += 8;
            }
            
            // 0x17: RLA
            0x17 => {
                self.rla();
                self.cycles += 4;
            }
            
            // 0x18: JR n
            0x18 => {
                let offset = self.fetch_byte() as i8;
                self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                self.cycles += 12;
            }
            
            // 0x19: ADD HL, DE
            0x19 => {
                self.add_hl(self.registers.de());
                self.cycles += 8;
            }
            
            // 0x1A: LD A, (DE)
            0x1a => {
                self.registers.a = self.mmu.read_byte(self.registers.de());
                self.cycles += 8;
            }
            
            // 0x1B: DEC DE
            0x1b => {
                let val = self.registers.de().wrapping_sub(1);
                self.registers.set_de(val);
                self.cycles += 8;
            }
            
            // 0x1C: INC E
            0x1c => {
                self.registers.e = self.inc8(self.registers.e);
                self.cycles += 4;
            }
            
            // 0x1D: DEC E
            0x1d => {
                self.registers.e = self.dec8(self.registers.e);
                self.cycles += 4;
            }
            
            // 0x1E: LD E, n
            0x1e => {
                self.registers.e = self.fetch_byte();
                self.cycles += 8;
            }
            
            // 0x1F: RRA
            0x1f => {
                self.rra();
                self.cycles += 4;
            }
            
            // 0x20: JR NZ, n
            0x20 => {
                let offset = self.fetch_byte() as i8;
                if !self.registers.flag_z() {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
            }
            
            // 0x21: LD HL, nn
            0x21 => {
                let val = self.fetch_word();
                self.registers.set_hl(val);
                self.cycles += 12;
            }
            
            // 0x22: LD (HL+), A
            0x22 => {
                self.mmu.write_byte(self.registers.hl(), self.registers.a);
                let val = self.registers.hl().wrapping_add(1);
                self.registers.set_hl(val);
                self.cycles += 8;
            }
            
            // 0x23: INC HL
            0x23 => {
                let val = self.registers.hl().wrapping_add(1);
                self.registers.set_hl(val);
                self.cycles += 8;
            }
            
            // 0x24: INC H
            0x24 => {
                self.registers.h = self.inc8(self.registers.h);
                self.cycles += 4;
            }
            
            // 0x25: DEC H
            0x25 => {
                self.registers.h = self.dec8(self.registers.h);
                self.cycles += 4;
            }
            
            // 0x26: LD H, n
            0x26 => {
                self.registers.h = self.fetch_byte();
                self.cycles += 8;
            }
            
            // 0x27: DAA
            0x27 => {
                self.daa();
                self.cycles += 4;
            }
            
            // 0x28: JR Z, n
            0x28 => {
                let offset = self.fetch_byte() as i8;
                if self.registers.flag_z() {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
            }
            
            // 0x29: ADD HL, HL
            0x29 => {
                self.add_hl(self.registers.hl());
                self.cycles += 8;
            }
            
            // 0x2A: LD A, (HL+)
            0x2a => {
                self.registers.a = self.mmu.read_byte(self.registers.hl());
                let val = self.registers.hl().wrapping_add(1);
                self.registers.set_hl(val);
                self.cycles += 8;
            }
            
            // 0x2B: DEC HL
            0x2b => {
                let val = self.registers.hl().wrapping_sub(1);
                self.registers.set_hl(val);
                self.cycles += 8;
            }
            
            // 0x2C: INC L
            0x2c => {
                self.registers.l = self.inc8(self.registers.l);
                self.cycles += 4;
            }
            
            // 0x2D: DEC L
            0x2d => {
                self.registers.l = self.dec8(self.registers.l);
                self.cycles += 4;
            }
            
            // 0x2E: LD L, n
            0x2e => {
                self.registers.l = self.fetch_byte();
                self.cycles += 8;
            }
            
            // 0x2F: CPL
            0x2f => {
                self.registers.a = !self.registers.a;
                self.registers.set_flag_n(true);
                self.registers.set_flag_h(true);
                self.cycles += 4;
            }
            
            // 0x30: JR NC, n
            0x30 => {
                let offset = self.fetch_byte() as i8;
                if !self.registers.flag_c() {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
            }
            
            // 0x31: LD SP, nn
            0x31 => {
                self.registers.sp = self.fetch_word();
                self.cycles += 12;
            }
            
            // 0x32: LD (HL-), A
            0x32 => {
                self.mmu.write_byte(self.registers.hl(), self.registers.a);
                let val = self.registers.hl().wrapping_sub(1);
                self.registers.set_hl(val);
                self.cycles += 8;
            }
            
            // 0x33: INC SP
            0x33 => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.cycles += 8;
            }
            
            // 0x34: INC (HL)
            0x34 => {
                let val = self.mmu.read_byte(self.registers.hl());
                let result = self.inc8(val);
                self.mmu.write_byte(self.registers.hl(), result);
                self.cycles += 12;
            }
            
            // 0x35: DEC (HL)
            0x35 => {
                let val = self.mmu.read_byte(self.registers.hl());
                let result = self.dec8(val);
                self.mmu.write_byte(self.registers.hl(), result);
                self.cycles += 12;
            }
            
            // 0x36: LD (HL), n
            0x36 => {
                let val = self.fetch_byte();
                self.mmu.write_byte(self.registers.hl(), val);
                self.cycles += 12;
            }
            
            // 0x37: SCF
            0x37 => {
                self.registers.set_flag_n(false);
                self.registers.set_flag_h(false);
                self.registers.set_flag_c(true);
                self.cycles += 4;
            }
            
            // 0x38: JR C, n
            0x38 => {
                let offset = self.fetch_byte() as i8;
                if self.registers.flag_c() {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
            }
            
            // 0x39: ADD HL, SP
            0x39 => {
                self.add_hl(self.registers.sp);
                self.cycles += 8;
            }
            
            // 0x3A: LD A, (HL-)
            0x3a => {
                self.registers.a = self.mmu.read_byte(self.registers.hl());
                let val = self.registers.hl().wrapping_sub(1);
                self.registers.set_hl(val);
                self.cycles += 8;
            }
            
            // 0x3B: DEC SP
            0x3b => {
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.cycles += 8;
            }
            
            // 0x3C: INC A
            0x3c => {
                self.registers.a = self.inc8(self.registers.a);
                self.cycles += 4;
            }
            
            // 0x3D: DEC A
            0x3d => {
                self.registers.a = self.dec8(self.registers.a);
                self.cycles += 4;
            }
            
            // 0x3E: LD A, n
            0x3e => {
                self.registers.a = self.fetch_byte();
                self.cycles += 8;
            }
            
            // 0x3F: CCF
            0x3f => {
                self.registers.set_flag_n(false);
                self.registers.set_flag_h(false);
                self.registers.set_flag_c(!self.registers.flag_c());
                self.cycles += 4;
            }
            
            // 0x40-0x75, 0x77-0x7F: LD r,r'
            0x40..=0x75 | 0x77..=0x7f => {
                self.ld_rr(opcode);
            }
            
            // 0x76: HALT
            0x76 => {
                self.halted = true;
                self.cycles += 4;
            }
            
            // 0x80-0xBF: ALU operations
            0x80..=0xbf => {
                self.alu_op(opcode);
            }
            
            // 0xCB: CB prefix
            0xcb => {
                let cb_opcode = self.fetch_byte();
                self.execute_cb_opcode(cb_opcode);
            }
            
            // Extended opcodes (0xC0+)
            _ => {
                self.execute_extended_opcode(opcode);
            }
        }
    }

    // LD r,r' helper (0x40-0x7F)
    fn ld_rr(&mut self, opcode: u8) {
        let dst = (opcode >> 3) & 0x07;
        let src = opcode & 0x07;
        let value = self.get_reg8(src);
        self.set_reg8(dst, value);
        self.cycles += if src == 6 || dst == 6 { 8 } else { 4 };
    }
    
    // ALU ops helper (0x80-0xBF)
    fn alu_op(&mut self, opcode: u8) {
        let op = (opcode >> 3) & 0x07;
        let reg = opcode & 0x07;
        let value = self.get_reg8(reg);
        
        match op {
            0 => self.add8(value),
            1 => self.adc8(value),
            2 => self.sub8(value),
            3 => self.sbc8(value),
            4 => self.and8(value),
            5 => self.xor8(value),
            6 => self.or8(value),
            7 => self.cp8(value),
            _ => {}
        }
        
        self.cycles += if reg == 6 { 8 } else { 4 };
    }
    
    // Read 8-bit reg or (HL)
    fn get_reg8(&self, index: u8) -> u8 {
        match index {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => self.mmu.read_byte(self.registers.hl()),
            7 => self.registers.a,
            _ => 0,
        }
    }
    
    // Write 8-bit reg or (HL)
    fn set_reg8(&mut self, index: u8, value: u8) {
        match index {
            0 => self.registers.b = value,
            1 => self.registers.c = value,
            2 => self.registers.d = value,
            3 => self.registers.e = value,
            4 => self.registers.h = value,
            5 => self.registers.l = value,
            6 => self.mmu.write_byte(self.registers.hl(), value),
            7 => self.registers.a = value,
            _ => {}
        }
    }
    
    // Extended opcodes (RET/CALL/JP/...)
    fn execute_extended_opcode(&mut self, opcode: u8) {
        match opcode {
            // 0xC0: RET NZ
            0xc0 => {
                if !self.registers.flag_z() {
                    self.registers.pc = self.pop_word();
                    self.cycles += 20;
                } else {
                    self.cycles += 8;
                }
            }
            // 0xC1: POP BC
            0xc1 => {
                let val = self.pop_word();
                self.registers.set_bc(val);
                self.cycles += 12;
            }
            // 0xC2: JP NZ, nn
            0xc2 => {
                let addr = self.fetch_word();
                if !self.registers.flag_z() {
                    self.registers.pc = addr;
                    self.cycles += 16;
                } else {
                    self.cycles += 12;
                }
            }
            // 0xC3: JP nn
            0xc3 => {
                self.registers.pc = self.fetch_word();
                self.cycles += 16;
            }
            // 0xC4: CALL NZ, nn
            0xc4 => {
                let addr = self.fetch_word();
                if !self.registers.flag_z() {
                    self.push_word(self.registers.pc);
                    self.registers.pc = addr;
                    self.cycles += 24;
                } else {
                    self.cycles += 12;
                }
            }
            // 0xC5: PUSH BC
            0xc5 => {
                self.push_word(self.registers.bc());
                self.cycles += 16;
            }
            // 0xC6: ADD A, n
            0xc6 => {
                let val = self.fetch_byte();
                self.add8(val);
                self.cycles += 8;
            }
            // 0xC7: RST 00H
            0xc7 => self.rst(0x00),
            // 0xC8: RET Z
            0xc8 => {
                if self.registers.flag_z() {
                    self.registers.pc = self.pop_word();
                    self.cycles += 20;
                } else {
                    self.cycles += 8;
                }
            }
            // 0xC9: RET
            0xc9 => {
                self.registers.pc = self.pop_word();
                self.cycles += 16;
            }
            // 0xCA: JP Z, nn
            0xca => {
                let addr = self.fetch_word();
                if self.registers.flag_z() {
                    self.registers.pc = addr;
                    self.cycles += 16;
                } else {
                    self.cycles += 12;
                }
            }
            // 0xCC: CALL Z, nn
            0xcc => {
                let addr = self.fetch_word();
                if self.registers.flag_z() {
                    self.push_word(self.registers.pc);
                    self.registers.pc = addr;
                    self.cycles += 24;
                } else {
                    self.cycles += 12;
                }
            }
            // 0xCD: CALL nn
            0xcd => {
                let addr = self.fetch_word();
                self.push_word(self.registers.pc);
                self.registers.pc = addr;
                self.cycles += 24;
            }
            // 0xCE: ADC A, n
            0xce => {
                let val = self.fetch_byte();
                self.adc8(val);
                self.cycles += 8;
            }
            // 0xCF: RST 08H
            0xcf => self.rst(0x08),
            // 0xD0: RET NC
            0xd0 => {
                if !self.registers.flag_c() {
                    self.registers.pc = self.pop_word();
                    self.cycles += 20;
                } else {
                    self.cycles += 8;
                }
            }
            // 0xD1: POP DE
            0xd1 => {
                let val = self.pop_word();
                self.registers.set_de(val);
                self.cycles += 12;
            }
            // 0xD2: JP NC, nn
            0xd2 => {
                let addr = self.fetch_word();
                if !self.registers.flag_c() {
                    self.registers.pc = addr;
                    self.cycles += 16;
                } else {
                    self.cycles += 12;
                }
            }
            // 0xD4: CALL NC, nn
            0xd4 => {
                let addr = self.fetch_word();
                if !self.registers.flag_c() {
                    self.push_word(self.registers.pc);
                    self.registers.pc = addr;
                    self.cycles += 24;
                } else {
                    self.cycles += 12;
                }
            }
            // 0xD5: PUSH DE
            0xd5 => {
                self.push_word(self.registers.de());
                self.cycles += 16;
            }
            // 0xD6: SUB n
            0xd6 => {
                let val = self.fetch_byte();
                self.sub8(val);
                self.cycles += 8;
            }
            // 0xD7: RST 10H
            0xd7 => self.rst(0x10),
            // 0xD8: RET C
            0xd8 => {
                if self.registers.flag_c() {
                    self.registers.pc = self.pop_word();
                    self.cycles += 20;
                } else {
                    self.cycles += 8;
                }
            }
            // 0xD9: RETI
            0xd9 => {
                self.registers.pc = self.pop_word();
                self.ime = true;
                self.cycles += 16;
            }
            // 0xDA: JP C, nn
            0xda => {
                let addr = self.fetch_word();
                if self.registers.flag_c() {
                    self.registers.pc = addr;
                    self.cycles += 16;
                } else {
                    self.cycles += 12;
                }
            }
            // 0xDC: CALL C, nn
            0xdc => {
                let addr = self.fetch_word();
                if self.registers.flag_c() {
                    self.push_word(self.registers.pc);
                    self.registers.pc = addr;
                    self.cycles += 24;
                } else {
                    self.cycles += 12;
                }
            }
            // 0xDE: SBC A, n
            0xde => {
                let val = self.fetch_byte();
                self.sbc8(val);
                self.cycles += 8;
            }
            // 0xDF: RST 18H
            0xdf => self.rst(0x18),
            // 0xE0: LDH (n), A
            0xe0 => {
                let offset = self.fetch_byte();
                self.mmu.write_byte(0xff00 | offset as u16, self.registers.a);
                self.cycles += 12;
            }
            // 0xE1: POP HL
            0xe1 => {
                let val = self.pop_word();
                self.registers.set_hl(val);
                self.cycles += 12;
            }
            // 0xE2: LD (C), A
            0xe2 => {
                self.mmu.write_byte(0xff00 | self.registers.c as u16, self.registers.a);
                self.cycles += 8;
            }
            // 0xE5: PUSH HL
            0xe5 => {
                self.push_word(self.registers.hl());
                self.cycles += 16;
            }
            // 0xE6: AND n
            0xe6 => {
                let val = self.fetch_byte();
                self.and8(val);
                self.cycles += 8;
            }
            // 0xE7: RST 20H
            0xe7 => self.rst(0x20),
            // 0xE8: ADD SP, n
            0xe8 => {
                let offset = self.fetch_byte() as i8;
                let sp = self.registers.sp;
                let result = sp.wrapping_add(offset as u16);
                self.registers.set_flag_z(false);
                self.registers.set_flag_n(false);
                self.registers.set_flag_h((sp & 0x0f) + ((offset as u16) & 0x0f) > 0x0f);
                self.registers.set_flag_c((sp & 0xff) + ((offset as u16) & 0xff) > 0xff);
                self.registers.sp = result;
                self.cycles += 16;
            }
            // 0xE9: JP (HL)
            0xe9 => {
                self.registers.pc = self.registers.hl();
                self.cycles += 4;
            }
            // 0xEA: LD (nn), A
            0xea => {
                let addr = self.fetch_word();
                self.mmu.write_byte(addr, self.registers.a);
                self.cycles += 16;
            }
            // 0xEE: XOR n
            0xee => {
                let val = self.fetch_byte();
                self.xor8(val);
                self.cycles += 8;
            }
            // 0xEF: RST 28H
            0xef => self.rst(0x28),
            // 0xF0: LDH A, (n)
            0xf0 => {
                let offset = self.fetch_byte();
                self.registers.a = self.mmu.read_byte(0xff00 | offset as u16);
                self.cycles += 12;
            }
            // 0xF1: POP AF
            0xf1 => {
                let val = self.pop_word();
                self.registers.set_af(val);
                self.cycles += 12;
            }
            // 0xF2: LD A, (C)
            0xf2 => {
                self.registers.a = self.mmu.read_byte(0xff00 | self.registers.c as u16);
                self.cycles += 8;
            }
            // 0xF3: DI
            0xf3 => {
                self.ime = false;
                self.cycles += 4;
            }
            // 0xF5: PUSH AF
            0xf5 => {
                self.push_word(self.registers.af());
                self.cycles += 16;
            }
            // 0xF6: OR n
            0xf6 => {
                let val = self.fetch_byte();
                self.or8(val);
                self.cycles += 8;
            }
            // 0xF7: RST 30H
            0xf7 => self.rst(0x30),
            // 0xF8: LD HL, SP+n
            0xf8 => {
                let offset = self.fetch_byte() as i8;
                let sp = self.registers.sp;
                let result = sp.wrapping_add(offset as u16);
                self.registers.set_flag_z(false);
                self.registers.set_flag_n(false);
                self.registers.set_flag_h((sp & 0x0f) + ((offset as u16) & 0x0f) > 0x0f);
                self.registers.set_flag_c((sp & 0xff) + ((offset as u16) & 0xff) > 0xff);
                self.registers.set_hl(result);
                self.cycles += 12;
            }
            // 0xF9: LD SP, HL
            0xf9 => {
                self.registers.sp = self.registers.hl();
                self.cycles += 8;
            }
            // 0xFA: LD A, (nn)
            0xfa => {
                let addr = self.fetch_word();
                self.registers.a = self.mmu.read_byte(addr);
                self.cycles += 16;
            }
            // 0xFB: EI
            0xfb => {
                self.ime_scheduled = true;
                self.cycles += 4;
            }
            // 0xFE: CP n
            0xfe => {
                let val = self.fetch_byte();
                self.cp8(val);
                self.cycles += 8;
            }
            // 0xFF: RST 38H
            0xff => self.rst(0x38),
            // Catch-all for undefined/illegal opcodes (should not normally be hit)
            _ => {
                // Just NOP for illegal ops to avoid infinite loops
                self.cycles += 4;
            }
        }
    }
    
    // CB-prefixed opcodes
    fn execute_cb_opcode(&mut self, opcode: u8) {
        let reg = opcode & 0x07;
        let bit = (opcode >> 3) & 0x07;
        let op = (opcode >> 6) & 0x03;
        
        if op == 0 {
            // Rot/shift
            let value = self.get_reg8(reg);
            let result = match bit {
                0 => self.rlc(value),
                1 => self.rrc(value),
                2 => self.rl(value),
                3 => self.rr(value),
                4 => self.sla(value),
                5 => self.sra(value),
                6 => self.swap(value),
                7 => self.srl(value),
                _ => value,
            };
            self.set_reg8(reg, result);
            self.cycles += if reg == 6 { 16 } else { 8 };
        } else if op == 1 {
            // BIT b,r
            let value = self.get_reg8(reg);
            self.registers.set_flag_z((value >> bit) & 1 == 0);
            self.registers.set_flag_n(false);
            self.registers.set_flag_h(true);
            self.cycles += if reg == 6 { 12 } else { 8 };
        } else if op == 2 {
            // RES b,r
            let value = self.get_reg8(reg);
            self.set_reg8(reg, value & !(1 << bit));
            self.cycles += if reg == 6 { 16 } else { 8 };
        } else {
            // SET b,r
            let value = self.get_reg8(reg);
            self.set_reg8(reg, value | (1 << bit));
            self.cycles += if reg == 6 { 16 } else { 8 };
        }
    }
    
    // ALU operations
    fn add8(&mut self, value: u8) {
        let result = self.registers.a.wrapping_add(value);
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h((self.registers.a & 0x0f) + (value & 0x0f) > 0x0f);
        self.registers.set_flag_c(self.registers.a as u16 + value as u16 > 0xff);
        self.registers.a = result;
    }
    
    fn adc8(&mut self, value: u8) {
        let carry = if self.registers.flag_c() { 1 } else { 0 };
        let result = self.registers.a.wrapping_add(value).wrapping_add(carry);
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h((self.registers.a & 0x0f) + (value & 0x0f) + carry > 0x0f);
        self.registers.set_flag_c(self.registers.a as u16 + value as u16 + carry as u16 > 0xff);
        self.registers.a = result;
    }
    
    fn sub8(&mut self, value: u8) {
        let result = self.registers.a.wrapping_sub(value);
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(true);
        self.registers.set_flag_h((self.registers.a & 0x0f) < (value & 0x0f));
        self.registers.set_flag_c(self.registers.a < value);
        self.registers.a = result;
    }
    
    fn sbc8(&mut self, value: u8) {
        let carry = if self.registers.flag_c() { 1 } else { 0 };
        let result = self.registers.a.wrapping_sub(value).wrapping_sub(carry);
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(true);
        self.registers.set_flag_h((self.registers.a & 0x0f) < (value & 0x0f) + carry);
        self.registers.set_flag_c((self.registers.a as u16) < (value as u16 + carry as u16));
        self.registers.a = result;
    }
    
    fn and8(&mut self, value: u8) {
        self.registers.a &= value;
        self.registers.set_flag_z(self.registers.a == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(true);
        self.registers.set_flag_c(false);
    }
    
    fn xor8(&mut self, value: u8) {
        self.registers.a ^= value;
        self.registers.set_flag_z(self.registers.a == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(false);
    }
    
    fn or8(&mut self, value: u8) {
        self.registers.a |= value;
        self.registers.set_flag_z(self.registers.a == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(false);
    }
    
    fn cp8(&mut self, value: u8) {
        let result = self.registers.a.wrapping_sub(value);
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(true);
        self.registers.set_flag_h((self.registers.a & 0x0f) < (value & 0x0f));
        self.registers.set_flag_c(self.registers.a < value);
    }
    
    fn inc8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h((value & 0x0f) == 0x0f);
        result
    }
    
    fn dec8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(true);
        self.registers.set_flag_h((value & 0x0f) == 0);
        result
    }
    
    fn add_hl(&mut self, value: u16) {
        let hl = self.registers.hl();
        let result = hl.wrapping_add(value);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h((hl & 0x0fff) + (value & 0x0fff) > 0x0fff);
        self.registers.set_flag_c(hl as u32 + value as u32 > 0xffff);
        self.registers.set_hl(result);
    }
    
    // Rotate/shift operations
    fn rlca(&mut self) {
        let carry = (self.registers.a >> 7) & 1;
        self.registers.a = (self.registers.a << 1) | carry;
        self.registers.set_flag_z(false);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(carry == 1);
    }
    
    fn rrca(&mut self) {
        let carry = self.registers.a & 1;
        self.registers.a = (self.registers.a >> 1) | (carry << 7);
        self.registers.set_flag_z(false);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(carry == 1);
    }
    
    fn rla(&mut self) {
        let carry = if self.registers.flag_c() { 1 } else { 0 };
        let new_carry = (self.registers.a >> 7) & 1;
        self.registers.a = (self.registers.a << 1) | carry;
        self.registers.set_flag_z(false);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(new_carry == 1);
    }
    
    fn rra(&mut self) {
        let carry = if self.registers.flag_c() { 1 } else { 0 };
        let new_carry = self.registers.a & 1;
        self.registers.a = (self.registers.a >> 1) | (carry << 7);
        self.registers.set_flag_z(false);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(new_carry == 1);
    }
    
    fn rlc(&mut self, value: u8) -> u8 {
        let carry = (value >> 7) & 1;
        let result = (value << 1) | carry;
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(carry == 1);
        result
    }
    
    fn rrc(&mut self, value: u8) -> u8 {
        let carry = value & 1;
        let result = (value >> 1) | (carry << 7);
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(carry == 1);
        result
    }
    
    fn rl(&mut self, value: u8) -> u8 {
        let carry = if self.registers.flag_c() { 1 } else { 0 };
        let new_carry = (value >> 7) & 1;
        let result = (value << 1) | carry;
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(new_carry == 1);
        result
    }
    
    fn rr(&mut self, value: u8) -> u8 {
        let carry = if self.registers.flag_c() { 1 } else { 0 };
        let new_carry = value & 1;
        let result = (value >> 1) | (carry << 7);
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(new_carry == 1);
        result
    }
    
    fn sla(&mut self, value: u8) -> u8 {
        let carry = (value >> 7) & 1;
        let result = value << 1;
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(carry == 1);
        result
    }
    
    fn sra(&mut self, value: u8) -> u8 {
        let carry = value & 1;
        let result = (value >> 1) | (value & 0x80);
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(carry == 1);
        result
    }
    
    fn srl(&mut self, value: u8) -> u8 {
        let carry = value & 1;
        let result = value >> 1;
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(carry == 1);
        result
    }
    
    fn swap(&mut self, value: u8) -> u8 {
        let result = (value >> 4) | (value << 4);
        self.registers.set_flag_z(result == 0);
        self.registers.set_flag_n(false);
        self.registers.set_flag_h(false);
        self.registers.set_flag_c(false);
        result
    }
    
    fn daa(&mut self) {
        let mut a = self.registers.a;
        if !self.registers.flag_n() {
            if self.registers.flag_c() || a > 0x99 {
                a = a.wrapping_add(0x60);
                self.registers.set_flag_c(true);
            }
            if self.registers.flag_h() || (a & 0x0f) > 0x09 {
                a = a.wrapping_add(0x06);
            }
        } else {
            if self.registers.flag_c() {
                a = a.wrapping_sub(0x60);
            }
            if self.registers.flag_h() {
                a = a.wrapping_sub(0x06);
            }
        }
        self.registers.a = a;
        self.registers.set_flag_z(a == 0);
        self.registers.set_flag_h(false);
    }
    
    fn rst(&mut self, addr: u16) {
        self.push_word(self.registers.pc);
        self.registers.pc = addr;
        self.cycles += 16;
    }

    pub fn frame_buffer_ptr(&self) -> *const u8 { 
        self.ppu.get_frame_buffer().as_ptr() 
    }
    
    pub fn frame_buffer_len(&self) -> usize { 
        self.ppu.get_frame_buffer().len() 
    }

    pub fn press_button(&mut self, bit: u8) {
        // Update internal input model (optional) and MMU's joypad state
        self.input.press_button(bit);
        self.mmu.joypad_press(bit);
    }

    pub fn release_button(&mut self, bit: u8) {
        self.input.release_button(bit);
        self.mmu.joypad_release(bit);
    }

    pub fn get_pc(&self) -> u16 {
        self.registers.pc
    }
    
    pub fn get_lcdc(&self) -> u8 {
        self.mmu.get_io()[0x40]
    }

    // Debug controls
    pub fn enable_trace(&mut self, enabled: bool) { self.trace_enabled = enabled; }

    pub fn dump_trace(&self) -> String {
        let mut out = String::new();
        use std::fmt::Write as _;
        let start = self.trace_idx.min(256);
        for i in 0..start {
            let (pc, op, sp) = self.trace_buf[(self.trace_idx.wrapping_sub(start - i)) & 0xff];
            let _ = write!(out, "{:04X}: {:02X} SP={:04X}\n", pc, op, sp);
        }
        if let Some((intr, pc, ie, if_)) = self.last_interrupt {
            let _ = write!(
                out,
                "Last interrupt: id={} pc={:04X} IE={:02X} IF={:02X}\nIME={}\n",
                intr,
                pc,
                ie,
                if_,
                self.ime
            );
        }
        out
    }

    pub fn save_state(&self) -> String {
        let state = SaveState {
            a: self.registers.a,
            f: self.registers.f,
            b: self.registers.b,
            c: self.registers.c,
            d: self.registers.d,
            e: self.registers.e,
            h: self.registers.h,
            l: self.registers.l,
            sp: self.registers.sp,
            pc: self.registers.pc,
            cycles: self.cycles,
        };
        serde_json::to_string(&state).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn load_state(&mut self, s: &str) {
        if let Ok(st) = serde_json::from_str::<SaveState>(s) {
            self.registers.a = st.a;
            self.registers.f = st.f;
            self.registers.b = st.b;
            self.registers.c = st.c;
            self.registers.d = st.d;
            self.registers.e = st.e;
            self.registers.h = st.h;
            self.registers.l = st.l;
            self.registers.sp = st.sp;
            self.registers.pc = st.pc;
            self.cycles = st.cycles;
        }
    }
}

// Free-function API to avoid Rc/RefMutFromWasmAbi on methods
#[wasm_bindgen]
pub fn gb_create() {
    GB_SINGLETON.with(|cell| {
        *cell.borrow_mut() = Some(GameBoy::new());
    });
}

#[wasm_bindgen]
pub fn gb_load_rom(data: &[u8]) {
    GB_SINGLETON.with(|cell| {
        if let Some(gb) = cell.borrow_mut().as_mut() {
            gb.load_rom(data);
        }
    });
}

#[wasm_bindgen]
pub fn gb_reset() {
    GB_SINGLETON.with(|cell| {
        if let Some(gb) = cell.borrow_mut().as_mut() { gb.reset(); }
    });
}

#[wasm_bindgen]
pub fn gb_start() {
    GB_SINGLETON.with(|cell| {
        if let Some(gb) = cell.borrow_mut().as_mut() { gb.start(); }
    });
}

#[wasm_bindgen]
pub fn gb_stop() {
    GB_SINGLETON.with(|cell| {
        if let Some(gb) = cell.borrow_mut().as_mut() { gb.stop(); }
    });
}

#[wasm_bindgen]
pub fn gb_is_running() -> bool {
    GB_SINGLETON.with(|cell| cell.borrow().as_ref().map(|g| g.is_running()).unwrap_or(false))
}

#[wasm_bindgen]
pub fn gb_run_frame() -> bool {
    GB_SINGLETON.with(|cell| {
        let mut_ref = &mut *cell.borrow_mut();
        if let Some(gb) = mut_ref.as_mut() { gb.run_frame() } else { false }
    })
}

#[wasm_bindgen]
pub fn gb_frame_buffer_ptr() -> *const u8 {
    GB_SINGLETON.with(|cell| {
        if let Some(gb) = cell.borrow().as_ref() { gb.frame_buffer_ptr() } else { std::ptr::null() }
    })
}

#[wasm_bindgen]
pub fn gb_frame_buffer_len() -> usize {
    GB_SINGLETON.with(|cell| {
        if let Some(gb) = cell.borrow().as_ref() { gb.frame_buffer_len() } else { 0 }
    })
}

#[wasm_bindgen]
pub fn screen_width() -> usize { SCREEN_WIDTH }

#[wasm_bindgen]
pub fn screen_height() -> usize { SCREEN_HEIGHT }

// Initialize better panic messages in the browser console
#[wasm_bindgen(start)]
pub fn wasm_start() {
    // Set panic hook for readable errors in JS console
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn gb_press_button(bit: u8) {
    GB_SINGLETON.with(|cell| {
        if let Some(gb) = cell.borrow_mut().as_mut() { gb.press_button(bit); }
    });
}

#[wasm_bindgen]
pub fn gb_release_button(bit: u8) {
    GB_SINGLETON.with(|cell| {
        if let Some(gb) = cell.borrow_mut().as_mut() { gb.release_button(bit); }
    });
}

#[wasm_bindgen]
pub fn gb_save_state() -> String {
    GB_SINGLETON.with(|cell| {
        if let Some(gb) = cell.borrow().as_ref() { gb.save_state() } else { "{}".to_string() }
    })
}

#[wasm_bindgen]
pub fn gb_load_state(state: &str) {
    GB_SINGLETON.with(|cell| {
        if let Some(gb) = cell.borrow_mut().as_mut() { gb.load_state(state); }
    });
}