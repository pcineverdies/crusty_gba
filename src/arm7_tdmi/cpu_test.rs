#[cfg(test)]
mod cpu_test {

    use crate::arm7_tdmi::{OperatingMode, ARM7TDMI, NOP};
    use crate::bus::{BusSignal, MemoryResponse};
    use crate::common::BitOperation;
    use std::collections::HashMap;

    #[test]
    fn data_processing_test_1() {
        let mut cpu = ARM7TDMI::new();

        let instructions = HashMap::from([
            (0x08000000_u32, 0xe2821010_u32), // add r1, r2, 0x10 <------------| <- entry point
            (0x08000004_u32, 0xe1a02001_u32), // mov r2, r1                    |
            (0x08000008_u32, 0xe3a03011_u32), // mov r3, 0x11                  |
            (0x0800000c_u32, 0xe0033002_u32), // and r3, r3, r2                |
            (0x08000010_u32, 0xe083f001_u32), // add pc r3, r1 -> b 0x20 --|   |
            (0x00000020_u32, 0xe3a00003_u32), // mov r0, 3 <---------------|   |
            (0x00000024_u32, 0xe3a01001_u32), // mov r1, 1                     |
            (0x00000028_u32, 0xe0802501_u32), // add r2, r0, r1, lsl 10        |
            (0x0000002c_u32, 0xe0003001_u32), // and r3, r0, r1                |
            (0x00000030_u32, 0xe0404101_u32), // sub r4, r0, r1, lsl 2         |
            (0x00000034_u32, 0xe0205001_u32), // eor r5, r0, r1                |
            (0x00000038_u32, 0xe3a00002_u32), // mov r0, 2                     |
            (0x0000003c_u32, 0xe1a0fd00_u32), // lsl pc, r0, 0x1a -------------|
        ]);
        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..25 {
            let req = cpu.step(response);
            response.data = *instructions.get(&req.address).unwrap_or(&NOP);
        }

        assert_eq!(cpu.rf.get_register(0, 0), 2);
        assert_eq!(cpu.rf.get_register(1, 0), 1043);
        assert_eq!(cpu.rf.get_register(2, 0), 1043);
        assert_eq!(cpu.rf.get_register(3, 0), 17);
        assert_eq!(cpu.rf.get_register(4, 0), u32::MAX);
        assert_eq!(cpu.rf.get_register(5, 0), 2);
    }

    #[test]
    fn data_processing_test_2() {
        let mut cpu = ARM7TDMI::new();

        let instructions = HashMap::from([
            (0x08000000_u32, NOP),            // <- entry point
            (0x08000004_u32, 0xe3a00000_u32), // mov r0, 0
            (0x08000008_u32, 0xe3a0100a_u32), // mov r1, 10
            (0x0800000c_u32, 0xe1500001_u32), // yyy: cmp r0, r1 <--|
            (0x08000010_u32, 0xaa000001_u32), // bge xxx -----------|---|
            (0x08000014_u32, 0xe2800001_u32), // add r0, 1          |   |
            (0x08000018_u32, 0xeafffffb_u32), // b yyy -------------|   |
            (0x0800001c_u32, 0xeafffffe_u32), // xxx: b xxx <-----------|
                                              //
        ]);
        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..100 {
            let req = cpu.step(response);
            response.data = *instructions.get(&req.address).unwrap_or(&NOP);
        }

        assert_eq!(cpu.rf.get_register(0, 0), 10);
    }

    #[test]
    fn branch_test() {
        let mut cpu = ARM7TDMI::new();

        let instructions = HashMap::from([
            (0x08000000_u32, 0xea00002e_u32), // 0x08000000: b 0x080000c0---| <- entry point
            (0x080000c0_u32, 0xe2811001_u32), // 0x080000c0: add r1, 1  <---+
            (0x080000c4_u32, 0xe2822002_u32), // 0x080000c4: add r2, 2      |
            (0x080000c8_u32, 0xe2833003_u32), // 0x080000c8: add r3, 3      |
            (0x080000cc_u32, 0xe2844004_u32), // 0x080000cc: add r4, 4      |
            (0x080000d0_u32, 0xeafffffa_u32), // 0x080000d0: b 0x080000c0 --|
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

        assert_eq!(cpu.rf.get_register(1, 0), 10 * 1);
        assert_eq!(cpu.rf.get_register(2, 0), 10 * 2);
        assert_eq!(cpu.rf.get_register(3, 0), 10 * 3);
        assert_eq!(cpu.rf.get_register(4, 0), 10 * 4);
    }

    #[test]
    fn load_store_test() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x00000004_u32, 0x17),
            (0x00000008_u32, 0x20),
            (0x0000000c_u32, 0x06000000_u32),
            (0x00000030_u32, 0xaabbccdd_u32),
            (0x06000000_u32, 0xe5904008_u32), // ldr r4, [r0, 8] <-------
            (0x06000004_u32, 0xe5804000_u32), // str r4, [r0, 0]        |
            (0x06000008_u32, 0xe5d0a031_u32), // ldrb r10, [r0, 0x30]   |
            (0x0600000c_u32, 0xeafffffe_u32), // b .                    |
            (0x08000000_u32, NOP),            // <--- entry point       |
            (0x08000004_u32, 0xe5907004_u32), // ldr r7, [r0, 4]        |
            (0x08000008_u32, 0xe590f00c_u32), // ldr r15, [r0, c] -------
            (0x0800000c_u32, 0xeafffffe_u32), // b .              -> never reached
        ]);
        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..30 {
            println!("Executing {:#08x}", cpu.arm_current_execute);
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(4, 0), 0x20);
        assert_eq!(*instructions.get(&0).unwrap_or(&0), 0x20);
        assert_eq!(cpu.rf.get_register(10, 0), 0xcc);
    }

    #[test]
    fn load_store_hw_test() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x00000020_u32, 0xaabbccdd_u32),
            (0x00000028_u32, 0x00006000_u32),
            (0x00000040_u32, 0x11111111_u32),
            (0x00000044_u32, 0x22222222_u32),
            (0x00006000_u32, NOP), //            <------------------------------|
            (0x00006004_u32, 0xe1c103b0_u32), // strh   r0, [r1, 0x30]          |
            (0x00006008_u32, 0xe1c124d0_u32), // ldrd   r2, [r1, 0x40]          |
            (0x0000600c_u32, 0xe3a080cb_u32), // mov r8, 203                    |
            (0x00006010_u32, 0xe3a090cc_u32), // mov r9, 204                    |
            (0x00006014_u32, 0xe1c185f0_u32), // strd r8, [r1, 0x50]            |
            (0x00006018_u32, 0xeafffffe_u32), // b .                            |
            (0x08000000_u32, NOP), //            <---- entry point              |
            (0x08000004_u32, 0xe1d0a2b0_u32), // ldrh   r10, [r0, 0x20]         |
            (0x08000008_u32, 0xe1d0b2f2_u32), // ldrsh  r11, [r0, 0x22]         |
            (0x0800000c_u32, 0xe1f0c2d3_u32), // ldrsb!  r12, [r0, 0x23]        |
            (0x08000010_u32, 0xe1d0f0f5_u32), // ldrsh   r15, [r0, 0x05] -------|
            (0x08000014_u32, 0xeafffffe_u32), // b . -> never reached
        ]);
        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..50 {
            println!("Executing {:#08x}", cpu.arm_current_execute);
            let req = cpu.step(response);
            println!(".. Requiring {:#08x}", req.address);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP);
            } else {
                instructions.insert(req.address, req.data);
            }
            println!("r8 is {}", cpu.rf.get_register(8, 0));
            println!("r9 is {}", cpu.rf.get_register(9, 0));
        }

        assert_eq!(cpu.rf.get_register(10, 0), 0x0000ccdd_u32);
        assert_eq!(cpu.rf.get_register(11, 0), 0xffffaabb_u32);
        assert_eq!(cpu.rf.get_register(12, 0), 0xffffffaa_u32);
        assert_eq!(cpu.rf.get_register(0, 0), 0x23);
        assert!(cpu.rf.get_register(15, 0) < 0x08000000);
        assert_eq!(*instructions.get(&0x30).unwrap_or(&0), 0x00230023);
        assert_eq!(cpu.rf.get_register(2, 0), 0x11111111_u32);
        assert_eq!(cpu.rf.get_register(3, 0), 0x22222222_u32);
        assert_eq!(cpu.rf.get_register(8, 0), 203);
        assert_eq!(cpu.rf.get_register(9, 0), 204);
        assert_eq!(*instructions.get(&0x50).unwrap_or(&0), 203);
        assert_eq!(*instructions.get(&0x54).unwrap_or(&0), 204);
    }

    #[test]
    fn swi_test() {
        let mut cpu = ARM7TDMI::new();
        let mut found_supervisor = false;

        let mut instructions = HashMap::from([
            (0x00000004_u32, NOP),
            (0x00000008_u32, 0xE3A03003_u32),
            (0x0000000c_u32, NOP),
            (0x00000010_u32, 0xE1B0F00E_u32),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xEF000010_u32),
            (0x08000008_u32, 0xE3A0E00A_u32),
        ]);
        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..50 {
            let req = cpu.step(response);
            println!("Executed: {:#08x}", cpu.arm_current_execute);
            println!("Current 15: {:#08x}", cpu.rf.get_register(15, 0));
            println!("Current r14: {:#08x}", cpu.rf.get_register(14, 0));
            println!("Current mode: {:?}", cpu.rf.get_mode());
            println!("--------------");
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP);
            } else {
                instructions.insert(req.address, req.data);
            }

            if cpu.rf.get_mode() == OperatingMode::SUPERVISOR {
                found_supervisor = true;
            }
        }

        assert_eq!(found_supervisor, true);
        assert_eq!(cpu.rf.get_register(14, 0), 10);
        assert_eq!(cpu.rf.get_register(3, 0), 3);
        assert_eq!(cpu.rf.get_mode(), OperatingMode::USER);
    }

    #[test]
    fn und_test() {
        let mut cpu = ARM7TDMI::new();
        let mut found_undefined = false;

        let mut instructions = HashMap::from([
            (0x00000004_u32, NOP),
            (0x00000008_u32, 0xE3A03003_u32),
            (0x0000000c_u32, NOP),
            (0x00000010_u32, 0xE1B0F00E_u32),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE7000010_u32),
            (0x08000008_u32, 0xE3A0E00A_u32),
        ]);
        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..50 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP);
            } else {
                instructions.insert(req.address, req.data);
            }

            if cpu.rf.get_mode() == OperatingMode::UND {
                found_undefined = true;
            }
        }

        assert_eq!(found_undefined, true);
        assert_eq!(cpu.rf.get_register(14, 0), 10);
        assert_eq!(cpu.rf.get_register(3, 0), 3);
        assert_eq!(cpu.rf.get_mode(), OperatingMode::USER);
    }

    #[test]
    fn psr_mrs_test() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x00000004_u32, NOP),
            (0x00000008_u32, 0xe14fb000_u32),
            (0x0000000c_u32, NOP),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xe15a000b_u32),
            (0x08000008_u32, 0xe10fa000_u32),
            (0x0800000c_u32, 0xe14fb000_u32),
            (0x80000010_u32, 0xe7000010_u32),
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..20 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(10, 0), cpu.rf.get_cpsr());
    }

    #[test]
    fn psr_msr_test() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x00000004_u32, NOP),
            (0x00000008_u32, NOP),
            (0x0000000c_u32, NOP),
            (0x00000010_u32, NOP),
            (0x00000014_u32, NOP),
            (0x00000018_u32, NOP),
            (0x0000001c_u32, NOP),
            (0x00000020_u32, NOP),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xe15a000a_u32),
            (0x08000008_u32, 0xe10fb000_u32),
            (0x0800000c_u32, 0xe328f20f_u32),
            (0x08000010_u32, 0xe10fc000_u32),
            (0x08000014_u32, 0xe321f0f7_u32),
            (0x08000018_u32, 0xe10fb000_u32),
            (0x0800001c_u32, NOP),
            (0x08000020_u32, NOP),
            (0x08000024_u32, NOP),
            (0x08000028_u32, NOP),
            (0x0800002c_u32, NOP),
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..20 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(12, 0), cpu.rf.get_register(11, 0));
        assert!(cpu.rf.get_register(11, 0).get_range(31, 28) == 0xf);
    }

    #[test]
    fn swp_test() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x00000004_u32, 0xaabbccdd),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xe3a0a0ff_u32),
            (0x08000008_u32, 0xe3a00004_u32),
            (0x0800000c_u32, 0xe100109a_u32),
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..20 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(1, 0), 0xaabbccdd);
        assert_eq!(*instructions.get(&0x4).unwrap_or(&0), 0xff);
    }

    #[test]
    fn mul_test() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x08000000_u32, 0xe3a0af59_u32), // mov r10, 356
            (0x08000004_u32, 0xe3e0b00b_u32), // mov r11, -12
            (0x08000008_u32, 0xe00c0b9a_u32), // mul r12, r10, r11
            (0x0800000c_u32, 0xe02dba9c_u32), // mul r13, r12, r10, r11
            (0x08000010_u32, NOP),
            (0x08000014_u32, 0xe3a01482_u32), // mov r1, 0x82000000
            (0x08000018_u32, 0xe3a02011_u32), // mov r2, 0x11
            (0x0800001c_u32, 0xe0843291_u32), // umull r3, r4, r1, r2
            (0x08000020_u32, 0xe0c65291_u32), // smull r5, r6, r1, r2
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..40 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(12, 0), 0xffffef50);
        assert_eq!(cpu.rf.get_register(13, 0), 0xffe8cb34);
        assert_eq!(cpu.rf.get_register(3, 0), 0xa2000000);
        assert_eq!(cpu.rf.get_register(4, 0), 0x00000008);
        assert_eq!(cpu.rf.get_register(5, 0), 0xa2000000);
        assert_eq!(cpu.rf.get_register(6, 0), 0xfffffff7);
    }
}
