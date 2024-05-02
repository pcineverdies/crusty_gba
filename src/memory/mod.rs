use crate::{bus::TransferSize, common::BitOperation};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::Cursor;
use std::io::Read;

pub struct Memory {
    is_read_only: bool,
    data: Vec<u32>,
    init_address: u32,
    size: u32,
    name: String,
}

impl Memory {
    pub fn new(init_address: u32, size: u32, rom: bool, name: String) -> Self {
        let data = vec![0 as u32; (size >> 2) as usize];

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

        self.data[((address - self.init_address) >> 2) as usize]
    }

    pub fn read_byte(&self, address: u32) -> u32 {
        if address - self.init_address > self.size {
            panic!("Address is to valid while accessing {}", self.name);
        }

        let offset = address % 4;
        let data_to_return = self.data[((address - self.init_address) >> 2) as usize];
        data_to_return.get_range(offset * 8 + 7, offset * 8)
    }

    pub fn read_halfword(&self, address: u32) -> u32 {
        if address - self.init_address > self.size {
            panic!("Address is to valid while accessing {}", self.name);
        }

        let offset = address.is_bit_set(1) as u32;
        let data_to_return = self.data[((address - self.init_address) >> 2) as usize];
        data_to_return.get_range(offset * 16 + 15, offset * 16)
    }

    pub fn read_word(&self, address: u32) -> u32 {
        if address - self.init_address > self.size {
            panic!("Address is to valid while accessing {}", self.name);
        }

        self.data[((address - self.init_address) >> 2) as usize]
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
                let mut data_to_write = self.data[((address >> 2) - self.init_address) as usize];
                let mask = 0x000000ff << offset * 8;
                data_to_write &= !mask;
                data_to_write |= data & mask;
                self.data[((address - self.init_address) >> 2) as usize] = data_to_write;
            }
            TransferSize::HALFWORD => {
                let offset = address.is_bit_set(1) as u32;
                let mut data_to_write = self.data[((address >> 2) - self.init_address) as usize];
                let mask = 0x0000ffff << offset * 16;
                data_to_write &= !mask;
                data_to_write |= data & mask;
                self.data[((address - self.init_address) >> 2) as usize] = data_to_write;
            }
            TransferSize::WORD => {
                self.data[((address - self.init_address) >> 2) as usize] = data;
            }
        }
    }

    pub fn init_from_file(&mut self, file_name: &String) {
        let mut f =
            File::open(&file_name).expect("Unable to load file while initializing {self.name}");
        let metadata = std::fs::metadata(&file_name)
            .expect("Unable to read metadata while initializing {self.name}");

        let mut buffer: Vec<u8> = vec![0; metadata.len() as usize];
        f.read(&mut buffer)
            .expect("Buffer overflow while initializing {self.name}");

        let mut index: u32 = 0;
        while (index as usize) < buffer.len() {
            let mut rdr = Cursor::new(&buffer[(index as usize)..(index.wrapping_add(4) as usize)]);
            let data = rdr.read_u32::<LittleEndian>().unwrap();
            self.data[(index >> 2) as usize] = data;
            index += 4;
        }
    }
}

#[test]
fn test_memory() {
    let mut memory = Memory::new(0, 0x100000, false, String::from("test memory"));

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
