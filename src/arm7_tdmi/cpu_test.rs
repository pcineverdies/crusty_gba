#[cfg(test)]
use crate::arm7_tdmi::{ARM7TDMI, NOP};
use crate::bus::{BusSignal, MemoryRequest, MemoryResponse};
use std::collections::HashMap;

#[test]
fn data_processing_test() {
    let mut cpu = ARM7TDMI::new();

    let instructions = HashMap::from([
        (0x08000000_u32, 0xe2821010_u32),
        (0x08000004_u32, 0xe1a02001_u32),
        (0x08000008_u32, 0xe3a03011_u32),
        (0x0800000c_u32, 0xe0033002_u32),
        (0x08000010_u32, 0xe083f001_u32),
        (0x00000020_u32, 0xe3a00003_u32),
        (0x00000024_u32, 0xe3a01001_u32),
        (0x00000028_u32, 0xe0802501_u32),
        (0x0000002c_u32, 0xe0003001_u32),
        (0x00000030_u32, 0xe0404101_u32),
        (0x00000034_u32, 0xe0205001_u32),
        (0x00000038_u32, 0xe3a00002_u32),
        (0x0000003c_u32, 0xe1a0fd00_u32),
    ]);
    let mut response = MemoryResponse {
        data: NOP,
        n_wait: BusSignal::HIGH,
    };

    for _ in 0..25 {
        let req = cpu.step(response);
        response.data = *instructions.get(&req.address).unwrap_or(&NOP);
    }

    assert_eq!(cpu.rf.get_register(0), 2);
    assert_eq!(cpu.rf.get_register(1), 1043);
    assert_eq!(cpu.rf.get_register(2), 1043);
    assert_eq!(cpu.rf.get_register(3), 17);
    assert_eq!(cpu.rf.get_register(4), u32::MAX);
    assert_eq!(cpu.rf.get_register(5), 2);
}
