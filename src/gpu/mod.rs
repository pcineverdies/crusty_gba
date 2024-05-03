pub mod display;
use crate::gpu::display::Display;
use crate::bus::TransferSize;
use crate::common::BitOperation;
use crate::memory::Memory;

pub struct Gpu {
    pub vram: Memory,
    display: Display,
    dispcnt: u32,
    step_counter: u32,
}

impl Gpu {
    pub fn new() -> Self {
        let mut display = Display::new();
        display.clear(0xffffffff);
        Self {
            vram: Memory::new(0x06000000, 0x18000, false, String::from("VRAM")),
            display,
            dispcnt: 0,
            step_counter: 0,
        }
    }

    pub fn step(&mut self) {
        self.step_counter += 1;
        if self.step_counter < 279620 {
            return;
        }
        self.step_counter = 0;

        if self.dispcnt.get_range(2, 0) == 3 {
            let mut pixel_array = vec![0 as u8; (display::GBA_SCREEN_WIDTH * display::GBA_SCREEN_HEIGHT * 4) as usize];

            for i in (0..0x12c00).step_by(2) {
                let pixel = self.vram.read_halfword(0x06000000_u32.wrapping_add(i));
                let pixel_index = i >> 1;
                pixel_array[(pixel_index * 4 + 3) as usize] = pixel.get_range(4, 0) as u8 * 8;
                pixel_array[(pixel_index * 4 + 2) as usize] = pixel.get_range(9, 5) as u8 * 8;
                pixel_array[(pixel_index * 4 + 1) as usize] = pixel.get_range(14, 10) as u8 * 8;
                pixel_array[(pixel_index * 4 + 0) as usize] = 0xff;
            }

            self.display.update(&pixel_array);
        }
    }

    pub fn read(&self, address: u32, mas: TransferSize) -> u32 {
        if address >= 0x06000000 && address < 0x06018000 {
            return self.vram.read(address, mas);
        } else if address == 0x04000000 {
            return self.dispcnt;
        } else {
            todo!();
        }
    }

    pub fn write(&mut self, address: u32, data: u32, mas: TransferSize) {
        if address >= 0x06000000 && address < 0x06018000 {
            self.vram.write(address, data, mas);
        } else if address == 0x04000000 {
            self.dispcnt = data.get_range(15, 0);
        } else {
            todo!();
        }
    }
}
