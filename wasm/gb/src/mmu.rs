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

        // IO defaults
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
            return self.io[offset];
        }
        if self.is_gbc {
            if offset == 0x4f { return self.vram_bank as u8 | 0xfe; }
            if offset == 0x70 { return self.wram_bank as u8 | 0xf8; }
        }
        self.io[offset]
    }

    fn write_io(&mut self, addr: usize, val: u8) {
        let offset = addr - 0xff00;
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

    pub fn get_oam(&self) -> &[u8] { &self.oam }
    pub fn get_io(&self) -> &[u8] { &self.io }
    pub fn get_io_mut(&mut self) -> &mut [u8] { &mut self.io }
    pub fn is_gbc(&self) -> bool { self.is_gbc }
}
