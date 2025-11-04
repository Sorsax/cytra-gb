use crate::mmu::MMU;

// APU (simplified - just tracks timing)
pub struct APU {
    accum_cycles: u32,
}

impl APU {
    pub fn new() -> Self {
        Self {
            accum_cycles: 0,
        }
    }

    pub fn reset(&mut self) {
        self.accum_cycles = 0;
    }

    pub fn step(&mut self, mmu: &MMU, cycles: u32) {
        // Track timing and master enable
        self.accum_cycles = self.accum_cycles.wrapping_add(cycles);
        
        // Read NR52 (master enable)
        let _ = self.is_enabled(mmu);
        
        // Bound counter
        if self.accum_cycles > (1 << 20) {
            self.accum_cycles &= (1 << 20) - 1;
        }
    }

    // NR52 bit7: master enable
    pub fn is_enabled(&self, mmu: &MMU) -> bool {
        (mmu.read_byte(0xff26) & 0x80) != 0
    }
}
