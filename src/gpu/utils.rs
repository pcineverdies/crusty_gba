use crate::common::BitOperation;
use crate::gpu::gpu_modes::*;
use crate::gpu::*;

impl Gpu {
    pub fn display_pixel(&mut self, index: u32, color: u32) {
        self.display_array[(index * 4 + 3) as usize] = color.get_range(4, 0) as u8 * 8;
        self.display_array[(index * 4 + 2) as usize] = color.get_range(9, 5) as u8 * 8;
        self.display_array[(index * 4 + 1) as usize] = color.get_range(14, 10) as u8 * 8;
        self.display_array[(index * 4 + 0) as usize] = 0xff;
    }

    pub fn frame_init_addr(&self) -> u32 {
        if self.current_dispcnt.is_bit_set(4) {
            VRAM_FRAME_1
        } else {
            VRAM_FRAME_0
        }
    }
}
