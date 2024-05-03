pub mod display;
use crate::bus::TransferSize;
use crate::common::BitOperation;
use crate::gpu::display::Display;
use crate::memory::Memory;

pub struct Gpu {
    pub vram: Memory,
    pub palette_ram: Memory,
    pub oam: Memory,
    pub gpu_registers: Memory,
    h_counter: u32,
    v_counter: u32,
    dot_counter: u32,
    display: Display,
    display_array: Vec<u8>,
}

impl Gpu {
    pub fn new() -> Self {
        let mut display = Display::new();
        display.clear(0xffffffff);
        Self {
            vram: Memory::new(0x06000000, 0x18000, false, String::from("VRAM")),
            palette_ram: Memory::new(0x05000000, 0x400, false, String::from("PALETTE RAM")),
            oam: Memory::new(0x07000000, 0x400, false, String::from("OAM")),
            gpu_registers: Memory::new(0x04000000, 0x58, false, String::from("GPU REGISTERS")),
            display,
            h_counter: 0,
            v_counter: 0,
            dot_counter: 0,
            display_array: vec![
                0 as u8;
                (display::GBA_SCREEN_WIDTH * display::GBA_SCREEN_HEIGHT * 4)
                    as usize
            ],
        }
    }

    pub fn step(&mut self) {
        self.dot_counter += 1;

        if self.dot_counter != 4 {
            return;
        } else {
            self.dot_counter = 0;
        }

        let pixel_index = self.h_counter + self.v_counter * 240;

        if self.v_counter < 160 && self.h_counter < 240 {
            let dispcnt = self.gpu_registers.read_halfword(0x04000000);

            if dispcnt.get_range(2, 0) == 3 {
                let pixel = self
                    .vram
                    .read_halfword(0x06000000_u32.wrapping_add(pixel_index * 2));
                self.display_array[(pixel_index * 4 + 3) as usize] =
                    pixel.get_range(4, 0) as u8 * 8;
                self.display_array[(pixel_index * 4 + 2) as usize] =
                    pixel.get_range(9, 5) as u8 * 8;
                self.display_array[(pixel_index * 4 + 1) as usize] =
                    pixel.get_range(14, 10) as u8 * 8;
                self.display_array[(pixel_index * 4 + 0) as usize] = 0xff;
            }
        }

        self.h_counter += 1;

        if self.h_counter == 240 + 68 {
            self.h_counter = 0;
            self.v_counter += 1;
        }

        if self.v_counter == 160 + 68 {
            self.v_counter = 0;
            self.display.update(&self.display_array);
        }
    }

    pub fn read(&self, address: u32, mas: TransferSize) -> u32 {
        if address >= 0x06000000 && address < 0x06018000 {
            return self.vram.read(address, mas);
        } else if address >= 0x05000000 && address < 0x05000400 {
            return self.palette_ram.read(address, mas);
        } else if address >= 0x07000000 && address < 0x07000400 {
            return self.oam.read(address, mas);
        } else if address >= 0x04000000 && address < 0x04000058 {
            return self.gpu_registers.read(address, mas);
        } else {
            todo!();
        }
    }

    pub fn write(&mut self, address: u32, data: u32, mas: TransferSize) {
        if address >= 0x06000000 && address < 0x06018000 {
            self.vram.write(address, data, mas);
        } else if address >= 0x05000000 && address < 0x05000400 {
            self.palette_ram.write(address, data, mas);
        } else if address >= 0x07000000 && address < 0x07000400 {
            self.oam.write(address, data, mas);
        } else if address >= 0x04000000 && address < 0x04000058 {
            self.gpu_registers.write(address, data, mas);
        } else {
            todo!();
        }
    }
}
