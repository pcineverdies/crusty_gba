#[cfg(test)]
use crate::arm7_tdmi::{ARM7TDMI, NOP};
use crate::bus::{BusSignal, MemoryRequest, MemoryResponse};
use std::collections::HashMap;

#[test]
fn data_processing_test_1() {
    let mut cpu = ARM7TDMI::new();

    let instructions = HashMap::from([
        (0x08000000_u32, 0xe2821010_u32), // add r1, r2, 0x10 <-----------------|
        (0x08000004_u32, 0xe1a02001_u32), // mov r2, r1                         |
        (0x08000008_u32, 0xe3a03011_u32), // mov r3, 0x11                       |
        (0x0800000c_u32, 0xe0033002_u32), // and r3, r3, r2                     |
        (0x08000010_u32, 0xe083f001_u32), // add pc r3, r1 -> b 0x20 --|        |
        // .......................................................................
        (0x00000020_u32, 0xe3a00003_u32), // mov r0, 3 <---------------|        |
        (0x00000024_u32, 0xe3a01001_u32), // mov r1, 1                          |
        (0x00000028_u32, 0xe0802501_u32), // add r2, r0, r1, lsl 10             |
        (0x0000002c_u32, 0xe0003001_u32), // and r3, r0, r1                     |
        (0x00000030_u32, 0xe0404101_u32), // sub r4, r0, r1, lsl 2              |
        (0x00000034_u32, 0xe0205001_u32), // eor r5, r0, r1                     |
        (0x00000038_u32, 0xe3a00002_u32), // mov r0, 2                          |
        (0x0000003c_u32, 0xe1a0fd00_u32), // lsl pc, r0, 0x1a -> b 0x08000000 --|
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

#[test]
fn data_processing_test_2() {
    let mut cpu = ARM7TDMI::new();

    let instructions = HashMap::from([
        (0x08000000_u32, 0xe2821010_u32),
        (0x08000004_u32, 0xe3a00000_u32),
        (0x08000008_u32, 0xe3a0100a_u32),
        (0x0800000c_u32, 0xe1500001_u32),
        (0x08000010_u32, 0xaa000001_u32),
        (0x08000014_u32, 0xe2800001_u32),
        (0x08000018_u32, 0xeafffffb_u32),
        (0x0800001c_u32, 0xeafffffe_u32),
    ]);
    let mut response = MemoryResponse {
        data: NOP,
        n_wait: BusSignal::HIGH,
    };

    for _ in 0..100 {
        let req = cpu.step(response);
        response.data = *instructions.get(&req.address).unwrap_or(&NOP);
    }

    assert_eq!(cpu.rf.get_register(0), 10);
}

#[test]
fn branch_test() {
    let mut cpu = ARM7TDMI::new();

    let instructions = HashMap::from([
        (0x08000000_u32, 0xea00002e_u32), // 0x08000000: b 0x080000c0---|
        (0x080000c0_u32, 0xe2811001_u32), // 0x080000c0: add r1, 1  <---|
        (0x080000c4_u32, 0xe2822002_u32), // 0x080000c4: add r2, 2      |
        (0x080000c8_u32, 0xe2833003_u32), // 0x080000c8: add r3, 3      |
        (0x080000cc_u32, 0xe2844004_u32), // 0x080000cc: add r4, 4      |
        (0x080000d0_u32, 0xeafffffa_u32), // 0x080000d0: b 0x080000c0 --
    ]);

    let mut response = MemoryResponse {
        data: NOP,
        n_wait: BusSignal::HIGH,
    };

    // 10 iterations of the loop
    for _ in 0..5 + 10 * 7 {
        let req = cpu.step(response);
        response.data = *instructions.get(&req.address).unwrap_or(&NOP);
    }

    assert_eq!(cpu.rf.get_register(1), 10 * 1);
    assert_eq!(cpu.rf.get_register(2), 10 * 2);
    assert_eq!(cpu.rf.get_register(3), 10 * 3);
    assert_eq!(cpu.rf.get_register(4), 10 * 4);
}
