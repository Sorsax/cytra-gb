// Input
pub struct Input {
    buttons: u8,
}

impl Input {
    pub fn new() -> Self {
        Input { buttons: 0xff }
    }

    pub fn reset(&mut self) {
        self.buttons = 0xff;
    }

    pub fn press_button(&mut self, button: u8) {
        self.buttons &= !(1 << button);
    }

    pub fn release_button(&mut self, button: u8) {
        self.buttons |= 1 << button;
    }

    pub fn update_joypad(&self, io: &mut [u8]) {
        let joyp = io[0x00];
        let mut new_joyp = joyp | 0x0f;

        // Group select
        if joyp & 0x10 == 0 {
            // D-pad
            new_joyp &= !((self.buttons >> 4) & 0x0f);
        }
        if joyp & 0x20 == 0 {
            // A/B/Select/Start
            new_joyp &= !(self.buttons & 0x0f);
        }

        io[0x00] = new_joyp;

        // Joypad IRQ if any pressed
        if new_joyp & 0x0f != 0x0f {
            io[0x0f] |= 0x10;
        }
    }
}
