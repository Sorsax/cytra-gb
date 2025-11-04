// CPU registers (Sharp LR35902)
#[derive(Clone)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub f: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            a: 0x01,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xd8,
            h: 0x01,
            l: 0x4d,
            f: 0xb0,
            sp: 0xfffe,
            pc: 0x0100,
        }
    }

    // Flags
    pub fn flag_z(&self) -> bool { self.f & 0x80 != 0 }
    pub fn set_flag_z(&mut self, v: bool) { self.f = if v { self.f | 0x80 } else { self.f & 0x7f }; }
    
    pub fn flag_n(&self) -> bool { self.f & 0x40 != 0 }
    pub fn set_flag_n(&mut self, v: bool) { self.f = if v { self.f | 0x40 } else { self.f & 0xbf }; }
    
    pub fn flag_h(&self) -> bool { self.f & 0x20 != 0 }
    pub fn set_flag_h(&mut self, v: bool) { self.f = if v { self.f | 0x20 } else { self.f & 0xdf }; }
    
    pub fn flag_c(&self) -> bool { self.f & 0x10 != 0 }
    pub fn set_flag_c(&mut self, v: bool) { self.f = if v { self.f | 0x10 } else { self.f & 0xef }; }

    // 16-bit pairs
    pub fn af(&self) -> u16 { (self.a as u16) << 8 | (self.f as u16) }
    pub fn set_af(&mut self, v: u16) { self.a = (v >> 8) as u8; self.f = (v & 0xf0) as u8; }
    
    pub fn bc(&self) -> u16 { (self.b as u16) << 8 | (self.c as u16) }
    pub fn set_bc(&mut self, v: u16) { self.b = (v >> 8) as u8; self.c = v as u8; }
    
    pub fn de(&self) -> u16 { (self.d as u16) << 8 | (self.e as u16) }
    pub fn set_de(&mut self, v: u16) { self.d = (v >> 8) as u8; self.e = v as u8; }
    
    pub fn hl(&self) -> u16 { (self.h as u16) << 8 | (self.l as u16) }
    pub fn set_hl(&mut self, v: u16) { self.h = (v >> 8) as u8; self.l = v as u8; }
}
