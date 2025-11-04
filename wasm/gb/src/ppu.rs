use crate::mmu::MMU;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

// PPU modes
const MODE_HBLANK: u8 = 0;
const MODE_VBLANK: u8 = 1;
const MODE_OAM_SCAN: u8 = 2;
const MODE_DRAWING: u8 = 3;

// Mode timings (T-cycles)
const MODE_OAM_CYCLES: u32 = 80;
const MODE_DRAWING_CYCLES: u32 = 172;
const SCANLINE_CYCLES: u32 = 456;

pub struct PPU {
    frame_buffer: Vec<u8>,
    scanline_counter: u32,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            frame_buffer: vec![0xff; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
            scanline_counter: 0,
        }
    }

    pub fn reset(&mut self, mmu: &mut MMU) {
        self.frame_buffer.fill(0xff);
        self.scanline_counter = 0;
        self.set_mode(mmu, MODE_OAM_SCAN);
        self.set_ly(mmu, 0);
    }

    pub fn get_frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    #[inline]
    fn set_pixel_rgb(&mut self, ly: u8, x: usize, rgb: [u8; 3]) {
        let idx = (ly as usize * SCREEN_WIDTH + x) * 4;
        if idx + 3 >= self.frame_buffer.len() {
            panic!(
                "PPU framebuffer overflow: ly={}, x={}, idx={}, len={}",
                ly,
                x,
                idx,
                self.frame_buffer.len()
            );
        }
        self.frame_buffer[idx] = rgb[0];
        self.frame_buffer[idx + 1] = rgb[1];
        self.frame_buffer[idx + 2] = rgb[2];
        self.frame_buffer[idx + 3] = 255;
    }

    // Step PPU; return true when a frame is ready
    pub fn step(&mut self, mmu: &mut MMU, cycles: u32) -> bool {
        let lcdc = mmu.get_io()[0x40];

        // LCD off?
        if (lcdc & 0x80) == 0 {
            return false;
        }

        self.scanline_counter += cycles;
        let ly = self.get_ly(mmu);

        // End of scanline
        if self.scanline_counter >= SCANLINE_CYCLES {
            self.scanline_counter -= SCANLINE_CYCLES;
            let new_ly = (ly + 1) % 154;
            self.set_ly(mmu, new_ly);

            // LYC=LY
            self.check_lyc(mmu);

            // VBlank
            if new_ly == 144 {
                self.set_mode(mmu, MODE_VBLANK);
                self.request_interrupt(mmu, 0); // VBlank interrupt
                return true; // Frame ready
            } else if new_ly == 0 {
                self.set_mode(mmu, MODE_OAM_SCAN);
            }
        }

        // Mode update
        if ly < 144 {
            if self.scanline_counter < MODE_OAM_CYCLES {
                self.set_mode(mmu, MODE_OAM_SCAN);
            } else if self.scanline_counter < MODE_OAM_CYCLES + MODE_DRAWING_CYCLES {
                if self.get_mode(mmu) != MODE_DRAWING {
                    self.set_mode(mmu, MODE_DRAWING);
                    self.render_scanline(mmu);
                }
            } else {
                if self.get_mode(mmu) != MODE_HBLANK {
                    self.set_mode(mmu, MODE_HBLANK);
                }
            }
        }

        false
    }

    fn render_scanline(&mut self, mmu: &MMU) {
        let lcdc = mmu.get_io()[0x40];
        let ly = self.get_ly(mmu);

        if ly >= SCREEN_HEIGHT as u8 {
            return;
        }

        // Clear line (white)
        let line_start = ly as usize * SCREEN_WIDTH * 4;
        for x in 0..SCREEN_WIDTH {
            let offset = line_start + x * 4;
            if offset + 3 >= self.frame_buffer.len() {
                panic!(
                    "PPU framebuffer clear overflow: ly={}, x={}, offset={}, len={}",
                    ly,
                    x,
                    offset,
                    self.frame_buffer.len()
                );
            }
            self.frame_buffer[offset] = 255;
            self.frame_buffer[offset + 1] = 255;
            self.frame_buffer[offset + 2] = 255;
            self.frame_buffer[offset + 3] = 255;
        }

        // BG (re-enabled for isolation test)
        if lcdc & 0x01 != 0 {
            self.render_background(mmu, ly);
        }

        // WIN
        if false && lcdc & 0x20 != 0 {
            self.render_window(mmu, ly);
        }

        // OBJ
        // TEMPORARY: Disable sprites to isolate memory corruption
        if false && lcdc & 0x02 != 0 {
            self.render_sprites(mmu, ly);
        }
    }

    fn render_background(&mut self, mmu: &MMU, ly: u8) {
        let io = mmu.get_io();
        let lcdc = io[0x40];
        let scy = io[0x42];
        let scx = io[0x43];
        let bgp = io[0x47];

        // Tile map/data
        let tile_map_base: u16 = if lcdc & 0x08 != 0 { 0x9c00 } else { 0x9800 };
        let tile_data_base: u16 = if lcdc & 0x10 != 0 { 0x8000 } else { 0x8800 };
        let signed_tile_data = (lcdc & 0x10) == 0;

        let y = ly.wrapping_add(scy);
        let tile_y = ((y >> 3) & 31) as u16;

        for x in 0..SCREEN_WIDTH {
            let x_pos = (x as u8).wrapping_add(scx);
            let tile_x = ((x_pos >> 3) & 31) as u16;
            let tile_index = tile_y * 32 + tile_x;

            // Tile number
            let tile_num = mmu.read_byte(tile_map_base + tile_index);
            let tile_addr = if signed_tile_data {
                let offset = (tile_num as i8 as i16 as u16).wrapping_add(128);
                tile_data_base.wrapping_add(offset * 16)
            } else {
                tile_data_base + (tile_num as u16) * 16
            };

            let tile_line = ((y & 7) * 2) as u16;

            // Tile data
            let byte1 = mmu.read_byte(tile_addr + tile_line);
            let byte2 = mmu.read_byte(tile_addr + tile_line + 1);

            // Pixel
            let bit_pos = 7 - (x_pos & 7);
            let color_num = ((byte2 >> bit_pos) & 1) << 1 | ((byte1 >> bit_pos) & 1);
            let color = (bgp >> (color_num * 2)) & 0x03;

            // Convert to RGB
            let rgb = self.get_color(color);
            self.set_pixel_rgb(ly, x, rgb);
        }
    }

    fn render_window(&mut self, mmu: &MMU, ly: u8) {
        let io = mmu.get_io();
        let lcdc = io[0x40];
        let wy = io[0x4a];
        let wx = io[0x4b];
        let bgp = io[0x47];

        if ly < wy {
            return;
        }

        let tile_map_base: u16 = if lcdc & 0x40 != 0 { 0x9c00 } else { 0x9800 };
        let tile_data_base: u16 = if lcdc & 0x10 != 0 { 0x8000 } else { 0x8800 };
        let signed_tile_data = (lcdc & 0x10) == 0;

        let window_y = ly - wy;
        let tile_y = ((window_y >> 3) & 31) as u16;

        for x in 0..SCREEN_WIDTH {
            let window_x = x as i16 - (wx as i16 - 7);
            if window_x < 0 {
                continue;
            }
            let window_x = window_x as u8;

            let tile_x = ((window_x >> 3) & 31) as u16;
            let tile_index = tile_y * 32 + tile_x;

            let tile_num = mmu.read_byte(tile_map_base + tile_index);
            let tile_addr = if signed_tile_data {
                let offset = (tile_num as i8 as i16 as u16).wrapping_add(128);
                tile_data_base.wrapping_add(offset * 16)
            } else {
                tile_data_base + (tile_num as u16) * 16
            };

            let tile_line = ((window_y & 7) * 2) as u16;

            let byte1 = mmu.read_byte(tile_addr + tile_line);
            let byte2 = mmu.read_byte(tile_addr + tile_line + 1);

            let bit_pos = 7 - (window_x & 7);
            let color_num = ((byte2 >> bit_pos) & 1) << 1 | ((byte1 >> bit_pos) & 1);
            let color = (bgp >> (color_num * 2)) & 0x03;

            let rgb = self.get_color(color);
            self.set_pixel_rgb(ly, x, rgb);
        }
    }

    fn render_sprites(&mut self, mmu: &MMU, ly: u8) {
        let io = mmu.get_io();
        let lcdc = io[0x40];
        let sprite_height = if lcdc & 0x04 != 0 { 16 } else { 8 };
        let oam = mmu.get_oam();

        // Collect sprites on this line into a small fixed buffer (avoid heap allocs)
        let mut buf: [(u8, usize); 10] = [(0, 0); 10];
        let mut count: usize = 0;
        for i in 0..40 {
            let sprite_y = oam[i * 4].wrapping_sub(16);
            if ly >= sprite_y && ly < sprite_y.wrapping_add(sprite_height) {
                if count < 10 {
                    buf[count] = (oam[i * 4 + 1], i);
                    count += 1;
                } else {
                    break; // max 10/line
                }
            }
        }

        // Sort by X, then index (simple insertion sort for small fixed buffer)
        for idx in 1..count {
            let mut j = idx;
            while j > 0 {
                let a = buf[j - 1];
                let b = buf[j];
                if a.0 > b.0 || (a.0 == b.0 && a.1 > b.1) {
                    buf[j - 1] = b;
                    buf[j] = a;
                    j -= 1;
                } else {
                    break;
                }
            }
        }

        // Render sprites
        for n in 0..count {
            let i = buf[n].1;
            let sprite_y = oam[i * 4].wrapping_sub(16);
            let sprite_x = oam[i * 4 + 1].wrapping_sub(8);
            let mut tile_num = oam[i * 4 + 2];
            let attributes = oam[i * 4 + 3];

            let palette = if attributes & 0x10 != 0 { io[0x49] } else { io[0x48] };
            let x_flip = (attributes & 0x20) != 0;
            let y_flip = (attributes & 0x40) != 0;
            let priority = (attributes & 0x80) != 0;

            // 8x16: ignore bit0
            if sprite_height == 16 {
                tile_num &= 0xfe;
            }

            let mut tile_line = ly.wrapping_sub(sprite_y);
            if y_flip {
                tile_line = sprite_height - 1 - tile_line;
            }

            let tile_addr = 0x8000 + (tile_num as u16) * 16 + (tile_line as u16) * 2;
            let byte1 = mmu.read_byte(tile_addr);
            let byte2 = mmu.read_byte(tile_addr + 1);

            for x in 0..8 {
                let screen_x = sprite_x.wrapping_add(x) as i16;
                if screen_x < 0 || screen_x >= SCREEN_WIDTH as i16 {
                    continue;
                }
                let screen_x = screen_x as usize;

                let bit_pos = if x_flip { x } else { 7 - x };
                let color_num = ((byte2 >> bit_pos) & 1) << 1 | ((byte1 >> bit_pos) & 1);

                // Color 0 = transparent
                if color_num == 0 {
                    continue;
                }

                // Priority
                if priority {
                    let offset = (ly as usize * SCREEN_WIDTH + screen_x) * 4;
                    // Bounds check before accessing framebuffer
                    if offset < self.frame_buffer.len() {
                        let bg_color = self.frame_buffer[offset];
                        if bg_color != 255 {
                            continue; // BG wins
                        }
                    }
                }

                let color = (palette >> (color_num * 2)) & 0x03;
                let rgb = self.get_color(color);
                self.set_pixel_rgb(ly, screen_x, rgb);
            }
        }
    }

    fn get_color(&self, color: u8) -> [u8; 3] {
        // DMG palette (green shades)
        match color & 0x03 {
            0 => [224, 248, 208], // White
            1 => [136, 192, 112], // Light gray
            2 => [52, 104, 86],   // Dark gray
            _ => [8, 24, 32],     // Black
        }
    }

    fn get_ly(&self, mmu: &MMU) -> u8 {
        mmu.get_io()[0x44]
    }

    fn set_ly(&self, mmu: &mut MMU, value: u8) {
        mmu.get_io_mut()[0x44] = value;
    }

    fn get_mode(&self, mmu: &MMU) -> u8 {
        mmu.get_io()[0x41] & 0x03
    }

    fn set_mode(&self, mmu: &mut MMU, mode: u8) {
        let stat = mmu.get_io()[0x41];
        let stat_interrupt_enabled = if mode != MODE_VBLANK {
            (stat >> (mode + 3)) & 1
        } else {
            0
        };
        
        mmu.get_io_mut()[0x41] = (stat & 0xfc) | (mode & 0x03);

        // STAT interrupt if enabled
        if stat_interrupt_enabled != 0 {
            self.request_interrupt(mmu, 1); // LCD STAT interrupt
        }
    }

    fn check_lyc(&self, mmu: &mut MMU) {
        let ly = mmu.get_io()[0x44];
        let lyc = mmu.get_io()[0x45];
        let stat = mmu.get_io()[0x41];

        // LY=LYC flag
        if ly == lyc {
            mmu.get_io_mut()[0x41] = stat | 0x04;
            // STAT if enabled
            if stat & 0x40 != 0 {
                self.request_interrupt(mmu, 1);
            }
        } else {
            mmu.get_io_mut()[0x41] = stat & 0xfb;
        }
    }

    fn request_interrupt(&self, mmu: &mut MMU, interrupt: u8) {
        let if_ = mmu.read_byte(0xff0f);
        mmu.write_byte(0xff0f, if_ | (1 << interrupt));
    }
}
