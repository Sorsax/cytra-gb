// Timer
pub struct Timer {
    div_counter: u32,
    tima_counter: u32,
}

impl Timer {
    pub fn new() -> Self {
        Timer { div_counter: 0, tima_counter: 0 }
    }

    pub fn reset(&mut self) {
        self.div_counter = 0;
        self.tima_counter = 0;
    }

    pub fn step(&mut self, cycles: u32, io: &mut [u8]) {
        // DIV @16384Hz
        self.div_counter += cycles;
        if self.div_counter >= 256 {
            self.div_counter -= 256;
            io[0x04] = io[0x04].wrapping_add(1);
        }

        // TIMA if enabled
        let tac = io[0x07];
        if tac & 0x04 != 0 {
            let frequencies = [1024, 16, 64, 256];
            let frequency = frequencies[(tac & 0x03) as usize];

            self.tima_counter += cycles;
            if self.tima_counter >= frequency {
                self.tima_counter -= frequency;

                let tima = io[0x05];
                if tima == 0xff {
                    // Overflow -> timer interrupt
                    let tma = io[0x06];
                    io[0x05] = tma;
                    io[0x0f] |= 0x04;
                } else {
                    io[0x05] = tima.wrapping_add(1);
                }
            }
        }
    }
}
