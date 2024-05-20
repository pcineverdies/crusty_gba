pub mod display;
pub mod gpu_modes;
pub mod utils;
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
    current_dispcnt: u32,
}

pub const V_SIZE: u32 = 160;
pub const H_SIZE: u32 = 240;

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
            current_dispcnt: 0,
        }
    }

    pub fn step(&mut self) {
        let mut dispstat = self.gpu_registers.read_halfword(0x04000004);

        self.dot_counter += 1;

        if self.dot_counter != 4 {
            return;
        }

        self.dot_counter = 0;

        if self.v_counter < V_SIZE && self.h_counter < H_SIZE {
            self.current_dispcnt = self.gpu_registers.read_halfword(0x04000000);

            if self.current_dispcnt.get_range(2, 0) == 3 {
                self.gpu_mode_3();
            } else if self.current_dispcnt.get_range(2, 0) == 4 {
                self.gpu_mode_4();
            } else if self.current_dispcnt.get_range(2, 0) == 5 {
                self.gpu_mode_5();
            }
        }

        self.h_counter += 1;

        if self.h_counter == H_SIZE + 68 {
            self.h_counter = 0;
            self.v_counter += 1;
        }

        if self.v_counter == V_SIZE + 68 {
            self.v_counter = 0;
            self.display.update(&self.display_array);
        }

        if self.v_counter >= 160 && self.v_counter < 227 {
            dispstat = dispstat.set_bit(0);
        } else {
            dispstat = dispstat.clear_bit(0);
        }

        if self.h_counter >= 251 {
            dispstat = dispstat.set_bit(1);
        } else {
            dispstat = dispstat.clear_bit(1);
        }

        self.gpu_registers
            .write(0x04000006, self.v_counter << 16, TransferSize::HALFWORD);
        self.gpu_registers
            .write(0x04000004, dispstat, TransferSize::HALFWORD);
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

    pub fn write(&mut self, address: u32, data: u32, mut mas: TransferSize) {
        if address >= 0x04000000 && address < 0x04000058 {
            self.gpu_registers.write(address, data, mas);
            return;
        }

        // VRAM, PRAM and OAM do not allow byte wrinting. When you try to do it,
        // they automatically become halfword writing
        if mas == TransferSize::BYTE {
            mas = TransferSize::HALFWORD;
        }

        if address >= 0x06000000 && address < 0x06018000 {
            self.vram.write(address, data, mas);
        } else if address >= 0x05000000 && address < 0x05000400 {
            self.palette_ram.write(address, data, mas);
        } else if address >= 0x07000000 && address < 0x07000400 {
            self.oam.write(address, data, mas);
        } else {
            todo!();
        }
    }
}
