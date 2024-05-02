extern crate num;
#[macro_use]
extern crate num_derive;
extern crate sdl2;
use std::env;
mod arm7_tdmi;
mod bus;
mod common;
mod gba;
mod gpu;
mod io;
mod memory;

fn main() {
    let mut gba = bus::Bus::new();
    let file_name = env::args().nth(1).expect("gba rom must be provided");
    gba.gamepak
        .init_from_file(&String::from(&file_name));

    loop {
        gba.step();
    }

}
