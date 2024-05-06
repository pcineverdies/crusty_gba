extern crate num;
#[macro_use]
extern crate num_derive;
extern crate sdl2;
use std::env;
mod arm7_tdmi;
mod bus;
mod common;
mod gpu;
mod memory;

fn main() {
    let mut gba = bus::Bus::new();
    let rom_file = env::args().nth(1).expect("gba rom must be provided");
    let bios_file = env::args().nth(2).expect("bios file must be provided");
    gba.gamepak.init_from_file(&String::from(&rom_file));
    gba.bios.init_from_file(&String::from(&bios_file));

    loop {
        gba.step();
    }
}
