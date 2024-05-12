use crate::bus::TransferSize;
use crate::common::BitOperation;
use crate::memory::Memory;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::Sdl;

pub struct Keypad {
    pub keypad_registers: Memory,
    sdl_context: Sdl,
}

impl Keypad {
    pub fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        Self {
            keypad_registers: Memory::new(0x04000130, 0x4, false, String::from("KEYPAD REGISTERS")),
            sdl_context,
        }
    }

    pub fn step(&mut self) {
        let keycnt = self.keypad_registers.read_halfword(0x04000132);
        let mut keyinput = 0xff;

        let mut events = self.sdl_context.event_pump().unwrap();

        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => std::process::exit(0),
                Event::KeyDown {
                    keycode: Some(Keycode::Backspace),
                    ..
                } => std::process::exit(1),
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => keyinput = keyinput.clear_bit(5),
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => keyinput = keyinput.clear_bit(7),
                Event::KeyDown {
                    keycode: Some(Keycode::D),
                    ..
                } => keyinput = keyinput.clear_bit(4),
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    ..
                } => keyinput = keyinput.clear_bit(6),
                Event::KeyDown {
                    keycode: Some(Keycode::J),
                    ..
                } => keyinput = keyinput.clear_bit(0),
                Event::KeyDown {
                    keycode: Some(Keycode::K),
                    ..
                } => keyinput = keyinput.clear_bit(1),
                Event::KeyDown {
                    keycode: Some(Keycode::V),
                    ..
                } => keyinput = keyinput.clear_bit(3),
                Event::KeyDown {
                    keycode: Some(Keycode::B),
                    ..
                } => keyinput = keyinput.clear_bit(4),
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    ..
                } => keyinput = keyinput.clear_bit(9),
                Event::KeyDown {
                    keycode: Some(Keycode::P),
                    ..
                } => keyinput = keyinput.clear_bit(8),
                _ => {}
            }
        }

        self.keypad_registers
            .write(0x04000130, keyinput, TransferSize::HALFWORD);
        self.keypad_registers
            .write(0x04000132, keycnt << 16, TransferSize::HALFWORD);
    }

    pub fn read(&self, address: u32, mas: TransferSize) -> u32 {
        if address >= 0x04000130 && address < 0x04000134 {
            return self.keypad_registers.read(address, mas);
        } else {
            todo!();
        }
    }

    pub fn write(&mut self, address: u32, data: u32, mas: TransferSize) {
        if address >= 0x04000132 && address < 0x04000134 {
            self.keypad_registers.write(address, data, mas);
        } else {
            todo!();
        }
    }
}
