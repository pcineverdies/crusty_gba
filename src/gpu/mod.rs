pub mod display;
use crate::bus::TransferSize;
use crate::common::BitOperation;
use crate::gpu::display::Display;
use crate::memory::Memory;

pub struct Gpu {
    pub vram: Memory,
    pub palette_ram: Memory,
    pub oam: Memory,
    step_counter: u32,
    display: Display,
    dispcnt: u32,
    green_swap: u32,
    dispstat: u32,
    vcount: u32,
    bg0cnt: u32,
    bg1cnt: u32,
    bg2cnt: u32,
    bg3cnt: u32,
}

impl Gpu {
    pub fn new() -> Self {
        let mut display = Display::new();
        display.clear(0xffffffff);
        Self {
            vram: Memory::new(0x06000000, 0x18000, false, String::from("VRAM")),
            palette_ram: Memory::new(0x05000000, 0x400, false, String::from("PALETTE RAM")),
            oam: Memory::new(0x07000000, 0x400, false, String::from("OAM")),
            step_counter: 0,
            display,
            dispcnt: 0,
            green_swap: 0,
            dispstat: 0,
            vcount: 0,
            bg0cnt: 0,
            bg1cnt: 0,
            bg2cnt: 0,
            bg3cnt: 0,
        }
    }

    pub fn step(&mut self) {
        self.step_counter += 1;

        if self.step_counter < 279620 {
            return;
        }

        self.step_counter = 0;

        let mut pixel_array =
            vec![0 as u8; (display::GBA_SCREEN_WIDTH * display::GBA_SCREEN_HEIGHT * 4) as usize];

        if self.dispcnt.get_range(2, 0) == 3 {
            for i in (0..0x12c00).step_by(2) {
                let pixel = self.vram.read_halfword(0x06000000_u32.wrapping_add(i));
                let pixel_index = i >> 1;
                pixel_array[(pixel_index * 4 + 3) as usize] = pixel.get_range(4, 0) as u8 * 8;
                pixel_array[(pixel_index * 4 + 2) as usize] = pixel.get_range(9, 5) as u8 * 8;
                pixel_array[(pixel_index * 4 + 1) as usize] = pixel.get_range(14, 10) as u8 * 8;
                pixel_array[(pixel_index * 4 + 0) as usize] = 0xff;
            }
        }
        self.display.update(&pixel_array);

    }

    pub fn read(&self, address: u32, mas: TransferSize) -> u32 {
        if address >= 0x06000000 && address < 0x06018000 {
            return self.vram.read(address, mas);
        } else if address >= 0x05000000 && address < 0x05000400 {
            return self.palette_ram.read(address, mas);
        } else if address >= 0x07000000 && address < 0x07000400 {
            return self.oam.read(address, mas);
        } else if address == 0x04000000 {
            return self.dispcnt;
        } else if address == 0x04000002 {
            return self.green_swap;
        } else if address == 0x04000004 {
            return self.dispstat;
        } else if address == 0x04000006 {
            return self.vcount;
        } else if address == 0x04000008 {
            return self.bg0cnt;
        } else if address == 0x0400000a {
            return self.bg1cnt;
        } else if address == 0x0400000c {
            return self.bg2cnt;
        } else if address == 0x0400000e {
            return self.bg3cnt;
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
        } else if address == 0x04000000 {
            self.dispcnt = data.get_range(15, 0);
        } else if address == 0x04000002 {
            self.green_swap = data.get_range(15, 0);
        } else if address == 0x04000004 {
            self.dispstat = data.get_range(15, 0);
        } else if address == 0x04000006 {
            // vcount is read only
        } else if address == 0x04000008 {
            self.bg0cnt = data.get_range(15, 0);
        } else if address == 0x0400000a {
            self.bg1cnt = data.get_range(15, 0);
        } else if address == 0x0400000c {
            self.bg2cnt = data.get_range(15, 0);
        } else if address == 0x0400000e {
            self.bg3cnt = data.get_range(15, 0);
        } else {
            todo!();
        }
    }
}
