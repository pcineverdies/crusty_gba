use crate::gpu::*;

pub const VRAM_INIT_ADDR: u32 = 0x06000000;
pub const PRAM_INIT_ADDR: u32 = 0x05000000;
pub const VRAM_FRAME_1: u32 = 0x0600A000;
pub const VRAM_FRAME_0: u32 = 0x06000000;

impl Gpu {
    pub fn gpu_mode_3(&mut self) {
        let pixel_index = self.h_counter + self.v_counter * H_SIZE;
        let pixel = self
            .vram
            .read_halfword(VRAM_INIT_ADDR.wrapping_add(pixel_index << 1));
        self.display_pixel(pixel_index, pixel);
    }

    pub fn gpu_mode_4(&mut self) {
        let pixel_index = self.h_counter + self.v_counter * H_SIZE;

        let init_address = self.frame_init_addr();

        let palette_color_address = self.vram.read_byte(init_address + pixel_index);
        let pixel = self
            .palette_ram
            .read_halfword(PRAM_INIT_ADDR | palette_color_address << 1);
        self.display_pixel(pixel_index, pixel);
    }

    pub fn gpu_mode_5(&mut self) {
        const MODE_5_H_SIZE: u32 = 160;
        const MODE_5_V_SIZE: u32 = 128;

        let pixel_index = self.h_counter + self.v_counter * H_SIZE;

        let pixel = if self.h_counter >= MODE_5_H_SIZE || self.v_counter >= MODE_5_V_SIZE {
            self.palette_ram.read_halfword(PRAM_INIT_ADDR)
        } else {
            let pixel_pointer = self.h_counter + self.v_counter * MODE_5_H_SIZE;
            let init_address = self.frame_init_addr();
            self.vram
                .read_halfword(init_address.wrapping_add(pixel_pointer << 1))
        };

        self.display_pixel(pixel_index, pixel);
    }
}
