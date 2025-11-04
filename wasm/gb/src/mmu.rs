// MMU: memory map, banking, I/O
pub struct MMU {
    rom: Vec<u8>,
    vram: Vec<u8>,
    eram: Vec<u8>,
    wram: Vec<u8>,
    oam: [u8; 0xa0],
    io: [u8; 0x80],
    hram: [u8; 0x7f],
    ie: u8,
    
    rom_bank: usize,
    ram_bank: usize,
    ram_enabled: bool,
    mbc_type: u8,
    banking_mode: u8,
    
    is_gbc: bool,
    vram_bank: usize,
    wram_bank: usize,
    vram_banks: [Vec<u8>; 2],
    wram_banks: Vec<Vec<u8>>,
    // CGB palette RAM and registers
    cgb_bg_palette_data: [u8; 64],
    cgb_obj_palette_data: [u8; 64],
    bgpi: u8,
    obpi: u8,
    // CGB HDMA (VRAM DMA)
    hdma_active: bool,
    hdma_hblank_mode: bool,
    hdma_src: u16,
    hdma_dst: u16,
    hdma_remaining: u16, // bytes remaining
    // Joypad state (active-low bits: 0=pressed)
    joypad_buttons: u8,
}

impl MMU {
    pub fn new() -> Self {
        let mut mmu = MMU {
            rom: vec![0; 0x8000],
            vram: vec![0; 0x2000],
            eram: vec![0; 0x2000],
            wram: vec![0; 0x2000],
            oam: [0; 0xa0],
            io: [0; 0x80],
            hram: [0; 0x7f],
            ie: 0,
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
            mbc_type: 0,
            banking_mode: 0,
            is_gbc: false,
            vram_bank: 0,
            wram_bank: 1,
            vram_banks: [vec![0; 0x2000], vec![0; 0x2000]],
            wram_banks: (0..8).map(|_| vec![0; 0x1000]).collect(),
            cgb_bg_palette_data: [0; 64],
            cgb_obj_palette_data: [0; 64],
            bgpi: 0,
            obpi: 0,
            hdma_active: false,
            hdma_hblank_mode: false,
            hdma_src: 0,
            hdma_dst: 0,
            hdma_remaining: 0,
            joypad_buttons: 0xff,
        };
        mmu.reset();
        mmu
    }

    pub fn reset(&mut self) {
        // Do NOT clear ROM here keep loaded cartridge contents intact across resets
        self.vram.fill(0);
        self.eram.fill(0);
        self.wram.fill(0);
        self.oam.fill(0);
        self.io.fill(0);
        self.hram.fill(0);
        self.ie = 0;
        self.rom_bank = 1;
        self.ram_bank = 0;
        self.ram_enabled = false;
        self.banking_mode = 0;
        self.vram_bank = 0;
        self.wram_bank = 1;
    self.cgb_bg_palette_data.fill(0);
    self.cgb_obj_palette_data.fill(0);
    self.bgpi = 0;
    self.obpi = 0;
        self.hdma_active = false;
        self.hdma_hblank_mode = false;
        self.hdma_src = 0;
        self.hdma_dst = 0;
        self.hdma_remaining = 0;
    self.joypad_buttons = 0xff;

        // IO defaults
    self.io[0x00] = 0xCF; // JOYP: no group selected, upper bits 1
        self.io[0x05] = 0x00; self.io[0x06] = 0x00; self.io[0x07] = 0x00;
        self.io[0x10] = 0x80; self.io[0x11] = 0xbf; self.io[0x12] = 0xf3; self.io[0x14] = 0xbf;
        self.io[0x16] = 0x3f; self.io[0x17] = 0x00; self.io[0x19] = 0xbf;
        self.io[0x1a] = 0x7f; self.io[0x1b] = 0xff; self.io[0x1c] = 0x9f; self.io[0x1e] = 0xbf;
        self.io[0x20] = 0xff; self.io[0x21] = 0x00; self.io[0x22] = 0x00; self.io[0x23] = 0xbf;
        self.io[0x24] = 0x77; self.io[0x25] = 0xf3; self.io[0x26] = 0xf1;
        self.io[0x40] = 0x91; self.io[0x42] = 0x00; self.io[0x43] = 0x00; self.io[0x45] = 0x00;
        self.io[0x47] = 0xfc; self.io[0x48] = 0xff; self.io[0x49] = 0xff;
        self.io[0x4a] = 0x00; self.io[0x4b] = 0x00;
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        let len = data.len().max(0x8000);
        self.rom = vec![0; len];
        self.rom[..data.len()].copy_from_slice(data);
        
        if data.len() > 0x0147 {
            self.mbc_type = data[0x0147];
            self.is_gbc = data.len() > 0x0143 && (data[0x0143] == 0x80 || data[0x0143] == 0xc0);
            
            if data.len() > 0x0149 {
                let ram_size = data[0x0149];
                let ram_sizes = [0, 0x800, 0x2000, 0x8000, 0x20000];
                if (ram_size as usize) < ram_sizes.len() {
                    self.eram = vec![0; ram_sizes[ram_size as usize]];
                }
            }
        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        match addr {
            0x0000..=0x3fff => self.rom.get(addr).copied().unwrap_or(0),
            0x4000..=0x7fff => {
                let offset = self.rom_bank * 0x4000 + (addr - 0x4000);
                self.rom.get(offset).copied().unwrap_or(0)
            }
            0x8000..=0x9fff => {
                let offset = addr - 0x8000;
                if self.is_gbc && self.vram_bank < 2 {
                    self.vram_banks[self.vram_bank].get(offset).copied().unwrap_or(0)
                } else if offset < self.vram.len() {
                    self.vram[offset]
                } else {
                    0
                }
            }
            0xa000..=0xbfff => {
                if self.ram_enabled {
                    let offset = self.ram_bank * 0x2000 + (addr - 0xa000);
                    self.eram.get(offset).copied().unwrap_or(0)
                } else {
                    0xff
                }
            }
            0xc000..=0xcfff => {
                let offset = addr - 0xc000;
                self.wram.get(offset).copied().unwrap_or(0)
            }
            0xd000..=0xdfff => {
                let offset = addr - 0xd000;
                if self.is_gbc && self.wram_bank < 8 {
                    self.wram_banks[self.wram_bank].get(offset).copied().unwrap_or(0)
                } else if offset < self.wram.len() {
                    self.wram[offset]
                } else {
                    0
                }
            }
            0xe000..=0xfdff => self.read_byte((addr - 0x2000) as u16),
            0xfe00..=0xfe9f => {
                let offset = addr - 0xfe00;
                if offset < self.oam.len() {
                    self.oam[offset]
                } else {
                    0xff
                }
            }
            0xfea0..=0xfeff => 0xff,
            0xff00..=0xff7f => self.read_io(addr),
            0xff80..=0xfffe => {
                let offset = addr - 0xff80;
                if offset < self.hram.len() {
                    self.hram[offset]
                } else {
                    0xff
                }
            }
            0xffff => self.ie,
            _ => 0xff,
        }
    }

    pub fn write_byte(&mut self, addr: u16, val: u8) {
        let addr = addr as usize;
        match addr {
            0x0000..=0x1fff => self.ram_enabled = (val & 0x0f) == 0x0a,
            0x2000..=0x3fff => {
                let mut bank = (val & 0x1f) as usize;
                if bank == 0 { bank = 1; }
                self.rom_bank = (self.rom_bank & 0x60) | bank;
            }
            0x4000..=0x5fff => {
                if self.banking_mode == 0 {
                    self.rom_bank = (self.rom_bank & 0x1f) | (((val & 0x03) as usize) << 5);
                } else {
                    self.ram_bank = (val & 0x03) as usize;
                }
            }
            0x6000..=0x7fff => self.banking_mode = val & 0x01,
            0x8000..=0x9fff => {
                let offset = addr - 0x8000;
                if self.is_gbc && self.vram_bank < 2 && offset < 0x2000 {
                    self.vram_banks[self.vram_bank][offset] = val;
                } else if offset < self.vram.len() {
                    self.vram[offset] = val;
                }
            }
            0xa000..=0xbfff => {
                if self.ram_enabled {
                    let offset = self.ram_bank * 0x2000 + (addr - 0xa000);
                    if offset < self.eram.len() {
                        self.eram[offset] = val;
                    }
                }
            }
            0xc000..=0xcfff => {
                let offset = addr - 0xc000;
                if offset < self.wram.len() {
                    self.wram[offset] = val;
                }
            }
            0xd000..=0xdfff => {
                let offset = addr - 0xd000;
                if self.is_gbc && self.wram_bank < 8 && offset < 0x1000 {
                    self.wram_banks[self.wram_bank][offset] = val;
                } else if offset < self.wram.len() {
                    self.wram[offset] = val;
                }
            }
            0xe000..=0xfdff => self.write_byte((addr - 0x2000) as u16, val),
            0xfe00..=0xfe9f => {
                let offset = addr - 0xfe00;
                if offset < self.oam.len() {
                    self.oam[offset] = val;
                }
            }
            0xfea0..=0xfeff => {}
            0xff00..=0xff7f => self.write_io(addr, val),
            0xff80..=0xfffe => {
                let offset = addr - 0xff80;
                if offset < self.hram.len() {
                    self.hram[offset] = val;
                }
            }
            0xffff => self.ie = val,
            _ => {}
        }
    }

    fn read_io(&self, addr: usize) -> u8 {
        let offset = addr - 0xff00;
        if offset == 0x00 {
            // JOYP read is dynamic based on select lines and current button state
            // Bits 6-7 read as 1; bits 4-5 are select lines; low nibble depends on selection
            let joyp = self.io[0x00];
            let mut value = 0xC0 | (joyp & 0x30) | 0x0F; // default: all released
            if joyp & 0x10 == 0 {
                // D-pad: Up/Down/Left/Right are bits 2/3/1/0 of upper nibble
                value &= !((self.joypad_buttons >> 4) & 0x0F);
            }
            if joyp & 0x20 == 0 {
                // Buttons: A/B/Select/Start are bits 0/1/2/3 of lower nibble
                value &= !(self.joypad_buttons & 0x0F);
            }
            return value;
        }
        if self.is_gbc {
            if offset == 0x4f { return self.vram_bank as u8 | 0xfe; }
            if offset == 0x70 { return self.wram_bank as u8 | 0xf8; }
            if offset == 0x68 { return self.bgpi; }
            if offset == 0x69 { return self.cgb_bg_palette_data[(self.bgpi & 0x3f) as usize]; }
            if offset == 0x6a { return self.obpi; }
            if offset == 0x6b { return self.cgb_obj_palette_data[(self.obpi & 0x3f) as usize]; }
            // HDMA registers
            if offset == 0x51 { return (self.hdma_src >> 8) as u8; }
            if offset == 0x52 { return (self.hdma_src & 0x00ff) as u8 & 0xF0; }
            if offset == 0x53 { return ((self.hdma_dst >> 8) as u8) & 0x1F; }
            if offset == 0x54 { return (self.hdma_dst & 0x00ff) as u8 & 0xF0; }
            if offset == 0x55 {
                // Bit7 indicates active when set; low 7 bits = remaining blocks-1
                if self.hdma_active {
                    let blocks = (self.hdma_remaining + 15) / 16;
                    return 0x80 | (((blocks.saturating_sub(1)) as u8) & 0x7f);
                } else {
                    return 0xff;
                }
            }
        }
        self.io[offset]
    }

    fn write_io(&mut self, addr: usize, val: u8) {
        let offset = addr - 0xff00;
        if offset == 0x00 {
            // JOYP: only bits 4-5 (select lines) are writable
            let prev = self.io[0x00];
            self.io[0x00] = (prev & 0xCF) | (val & 0x30);
            return;
        }
        if offset == 0x04 { self.io[offset] = 0; return; }
        if offset == 0x41 { self.io[offset] = (self.io[offset] & 0x07) | (val & 0xf8); return; }
        if offset == 0x44 { return; }
        if offset == 0x46 { self.dma_transfer(val); self.io[offset] = val; return; }
        if self.is_gbc {
            if offset == 0x4f { self.vram_bank = (val & 0x01) as usize; return; }
            if offset == 0x70 {
                let bank = (val & 0x07) as usize;
                self.wram_bank = if bank == 0 { 1 } else { bank };
                return;
            }
            // HDMA source/dest registers
            if offset == 0x51 { self.hdma_src = (self.hdma_src & 0x00ff) | ((val as u16) << 8); return; }
            if offset == 0x52 { self.hdma_src = (self.hdma_src & 0xff00) | (val as u16 & 0xF0); return; }
            if offset == 0x53 { self.hdma_dst = (self.hdma_dst & 0x00ff) | (((val as u16 & 0x1F) | 0x80) << 8); return; }
            if offset == 0x54 { self.hdma_dst = (self.hdma_dst & 0xff00) | (val as u16 & 0xF0); return; }
            if offset == 0x68 { self.bgpi = val & 0xbf; return; }
            if offset == 0x69 {
                let idx = (self.bgpi & 0x3f) as usize;
                self.cgb_bg_palette_data[idx] = val;
                if (self.bgpi & 0x80) != 0 { self.bgpi = (self.bgpi & 0x80) | ((self.bgpi.wrapping_add(1)) & 0x3f); }
                return;
            }
            if offset == 0x6a { self.obpi = val & 0xbf; return; }
            if offset == 0x6b {
                let idx = (self.obpi & 0x3f) as usize;
                self.cgb_obj_palette_data[idx] = val;
                if (self.obpi & 0x80) != 0 { self.obpi = (self.obpi & 0x80) | ((self.obpi.wrapping_add(1)) & 0x3f); }
                return;
            }
            if offset == 0x55 {
                // Length is (val & 0x7F) + 1 blocks of 16 bytes
                let blocks = ((val as u16 & 0x7f) + 1) as u16;
                let length = blocks * 16;
                if (val & 0x80) == 0 {
                    // General DMA: copy all at once
                    self.hdma_active = false;
                    self.do_hdma_copy(length);
                    self.io[0x55] = 0xff; // not active
                } else {
                    // HBlank DMA: start / update
                    self.hdma_active = true;
                    self.hdma_hblank_mode = true;
                    self.hdma_remaining = length;
                    // reflect remaining blocks (bit7 stays set)
                    self.io[0x55] = 0x80 | (((blocks - 1) as u8) & 0x7f);
                }
                return;
            }
        }
        self.io[offset] = val;
    }

    fn dma_transfer(&mut self, val: u8) {
        let src = (val as u16) << 8;
        for i in 0..0xa0 {
            self.oam[i] = self.read_byte(src + i as u16);
        }
    }

    pub fn get_vram(&self) -> &[u8] {
        if self.is_gbc { &self.vram_banks[self.vram_bank] } else { &self.vram }
    }

    pub fn get_vram_bank_ref(&self, bank: usize) -> &[u8] {
        if self.is_gbc { &self.vram_banks[bank & 1] } else { &self.vram }
    }

    pub fn read_vram_bank_byte(&self, addr: u16, bank: usize) -> u8 {
        let offset = addr as usize - 0x8000;
        if self.is_gbc {
            self.vram_banks[bank & 1].get(offset).copied().unwrap_or(0)
        } else {
            self.vram.get(offset).copied().unwrap_or(0)
        }
    }

    fn expand_5_to_8(v: u16) -> u8 { ((v * 527 + 23) >> 6) as u8 }

    pub fn cgb_get_bg_color_rgb(&self, palette: u8, index: u8) -> [u8; 3] {
        let idx = (palette as usize & 7) * 8 + (index as usize & 3) * 2;
        let lo = self.cgb_bg_palette_data[idx] as u16;
        let hi = self.cgb_bg_palette_data[idx + 1] as u16;
        let val = lo | (hi << 8);
        let r = Self::expand_5_to_8(val & 0x1f);
        let g = Self::expand_5_to_8((val >> 5) & 0x1f);
        let b = Self::expand_5_to_8((val >> 10) & 0x1f);
        [r, g, b]
    }

    pub fn cgb_get_obj_color_rgb(&self, palette: u8, index: u8) -> [u8; 3] {
        let idx = (palette as usize & 7) * 8 + (index as usize & 3) * 2;
        let lo = self.cgb_obj_palette_data[idx] as u16;
        let hi = self.cgb_obj_palette_data[idx + 1] as u16;
        let val = lo | (hi << 8);
        let r = Self::expand_5_to_8(val & 0x1f);
        let g = Self::expand_5_to_8((val >> 5) & 0x1f);
        let b = Self::expand_5_to_8((val >> 10) & 0x1f);
        [r, g, b]
    }

    pub fn get_oam(&self) -> &[u8] { &self.oam }
    pub fn get_io(&self) -> &[u8] { &self.io }
    pub fn get_io_mut(&mut self) -> &mut [u8] { &mut self.io }
    pub fn is_gbc(&self) -> bool { self.is_gbc }

    // Joypad updates from frontend
    pub fn joypad_press(&mut self, bit: u8) {
        self.joypad_buttons &= !(1 << bit);
        // Request joypad interrupt when any button down (simplified)
        self.io[0x0F] |= 0x10;
    }

    pub fn joypad_release(&mut self, bit: u8) {
        self.joypad_buttons |= 1 << bit;
    }

    // Perform one 16-byte HDMA chunk if active and in HBlank
    pub fn hdma_hblank_step(&mut self) {
        if !self.is_gbc || !self.hdma_active || !self.hdma_hblank_mode || self.hdma_remaining == 0 {
            return;
        }
        let to_copy = 16u16;
        self.do_hdma_copy(to_copy);
        if self.hdma_remaining == 0 {
            self.hdma_active = false;
            self.hdma_hblank_mode = false;
            self.io[0x55] = 0xff; // done
        } else {
            let blocks = (self.hdma_remaining + 15) / 16;
            self.io[0x55] = 0x80 | (((blocks - 1) as u8) & 0x7f);
        }
    }

    fn do_hdma_copy(&mut self, mut len: u16) {
        while len > 0 {
            let byte = self.read_byte(self.hdma_src);
            let dst_off = (self.hdma_dst as usize).saturating_sub(0x8000);
            if dst_off < 0x2000 {
                if self.is_gbc && self.vram_bank < 2 {
                    self.vram_banks[self.vram_bank][dst_off] = byte;
                } else {
                    if dst_off < self.vram.len() { self.vram[dst_off] = byte; }
                }
            }
            self.hdma_src = self.hdma_src.wrapping_add(1);
            self.hdma_dst = self.hdma_dst.wrapping_add(1);
            self.hdma_remaining = self.hdma_remaining.saturating_sub(1);
            len = len.saturating_sub(1);
        }
    }
}
