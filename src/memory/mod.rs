use crate::{bus::TransferSize, common::BitOperation};

pub struct Memory {
    is_read_only: bool,
    data: Vec<u32>,
    init_address: u32,
    size: u32,
    name: String,
}

impl Memory {
    pub fn new(init_address: u32, size: u32, rom: bool, name: String) -> Self {
        let data = vec![0; size as usize];

        Self {
            is_read_only: rom,
            data,
            init_address,
            size,
            name,
        }
    }

    pub fn read(&self, address: u32, mas: TransferSize) -> u32 {
        if address - self.init_address > self.size {
            panic!("Address is to valid while accessing {}", self.name);
        }

        // TODO: What happens for misaligned addresses?

        self.data[((address & 0xfffffffc) - self.init_address) as usize]
    }

    pub fn read_byte(&self, address: u32) -> u32 {
        if address - self.init_address > self.size {
            panic!("Address is to valid while accessing {}", self.name);
        }

        let offset = address % 4;
        let data_to_return = self.data[((address & 0xfffffffc) - self.init_address) as usize];
        data_to_return.get_range(offset * 8 + 7, offset * 8)
    }

    pub fn read_halfword(&self, address: u32) -> u32 {
        if address - self.init_address > self.size {
            panic!("Address is to valid while accessing {}", self.name);
        }

        let offset = address.is_bit_set(1) as u32;
        let data_to_return = self.data[((address & 0xfffffffc) - self.init_address) as usize];
        data_to_return.get_range(offset * 16 + 15, offset * 16)
    }

    pub fn read_word(&self, address: u32) -> u32 {
        if address - self.init_address > self.size {
            panic!("Address is to valid while accessing {}", self.name);
        }

        self.data[((address & 0xfffffffc) - self.init_address) as usize]
    }

    pub fn write(&mut self, address: u32, data: u32, mas: TransferSize) {
        if address - self.init_address > self.size {
            panic!("Address is to valid while accessing {}", self.name);
        }

        if self.is_read_only {
            return;
        }

        match mas {
            TransferSize::BYTE => {
                let offset = address % 4;
                let mut data_to_write =
                    self.data[((address & 0xfffffffc) - self.init_address) as usize];
                let mask = 0x000000ff << offset * 8;
                data_to_write &= !mask;
                data_to_write |= data & mask;
                self.data[((address & 0xfffffffc) - self.init_address) as usize] = data_to_write;
            }
            TransferSize::HALFWORD => {
                let offset = address.is_bit_set(1) as u32;
                let mut data_to_write =
                    self.data[((address & 0xfffffffc) - self.init_address) as usize];
                let mask = 0x0000ffff << offset * 16;
                data_to_write &= !mask;
                data_to_write |= data & mask;
                self.data[((address & 0xfffffffc) - self.init_address) as usize] = data_to_write;
            }
            TransferSize::WORD => {
                self.data[((address & 0xfffffffc) - self.init_address) as usize] = data;
            }
        }
    }
}

#[test]
fn test_memory() {
    let mut memory = Memory::new(0, 0x1000, false, String::from("test memory"));

    assert_eq!(memory.read(0, TransferSize::WORD), 0);
    memory.write(0, 0xaabbccdd, TransferSize::WORD);
    assert_eq!(memory.read(0, TransferSize::WORD), 0xaabbccdd);
    memory.write(0, 0x12341234, TransferSize::HALFWORD);
    assert_eq!(memory.read(0, TransferSize::WORD), 0xaabb1234);
    memory.write(0, 0, TransferSize::BYTE);
    assert_eq!(memory.read(0, TransferSize::WORD), 0xaabb1200);
    memory.write(6, 0x45674567, TransferSize::HALFWORD);
    assert_eq!(memory.read(4, TransferSize::WORD), 0x45670000);
    assert_eq!(memory.read(5, TransferSize::WORD), 0x45670000);
    assert_eq!(memory.read(6, TransferSize::WORD), 0x45670000);
    assert_eq!(memory.read(7, TransferSize::WORD), 0x45670000);
    memory.write(5, 0x12121212, TransferSize::BYTE);
    assert_eq!(memory.read(4, TransferSize::WORD), 0x45671200);
    assert_eq!(memory.read_byte(4), 0x00);
    assert_eq!(memory.read_byte(5), 0x12);
    assert_eq!(memory.read_byte(6), 0x67);
    assert_eq!(memory.read_byte(7), 0x45);
    assert_eq!(memory.read_halfword(4), 0x1200);
    assert_eq!(memory.read_halfword(6), 0x4567);
    assert_eq!(memory.read_word(6), 0x45671200);
}
