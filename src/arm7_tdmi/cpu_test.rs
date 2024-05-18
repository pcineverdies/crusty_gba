#[cfg(test)]
mod cpu_test {

    use crate::arm7_tdmi::{OperatingMode, ARM7TDMI, NOP, NOP_THUMB};
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
        assert_eq!(cpu.rf.get_mode(), OperatingMode::SYSTEM);
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
        assert_eq!(cpu.rf.get_mode(), OperatingMode::SYSTEM);
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

    #[test]
    fn stm_ldm_test() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xe3a0a010_u32), // mov r10, 0x10
            (0x08000008_u32, 0xe3a03003_u32), // mov r3, 0x3
            (0x0800000c_u32, 0xe3a04004_u32), // mov r4, 0x4
            (0x08000010_u32, 0xe3a07007_u32), // mov r7, 0x7
            (0x08000014_u32, 0xe82a0098_u32), // stmda r10!, {r3, r4, r7}
            (0x08000018_u32, 0xe99a3800_u32), // ldmib r10!, {r11, r12, r13}
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

        assert_eq!(*instructions.get(&0x10).unwrap_or(&0), 7);
        assert_eq!(*instructions.get(&0x0c).unwrap_or(&0), 4);
        assert_eq!(*instructions.get(&0x08).unwrap_or(&0), 3);
        assert_eq!(cpu.rf.get_register(10, 0), 0x4_u32);
        assert_eq!(cpu.rf.get_register(11, 0), 0x3_u32);
        assert_eq!(cpu.rf.get_register(12, 0), 0x4_u32);
        assert_eq!(cpu.rf.get_register(13, 0), 0x7_u32);
    }

    #[test]
    fn bx_test() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x00100000_u32, 0x2222_1111),
            (0x00100004_u32, 0x4444_3333),
            (0x00100008_u32, 0x6666_5555),
            (0x0010000c_u32, 0x8888_7777),
            (0x00100010_u32, 0xaaaa_9999),
            (0x00100014_u32, 0xcccc_bbbb),
            (0x00100018_u32, 0xffff_dddd),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE12FFF1A_u32),
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..8 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.arm_current_execute, 0x00001111);
    }

    #[test]
    fn thumb_move_shifted_register() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x00100000_u32, 0x014a_21ff), // mov r1, 0xff  --  lsl r2, r1, 5
            (0x00100004_u32, 0x114d_094b), // lsr r3, r1, 5 -- asr r5, r1, 5
            (0x00100008_u32, 0x1237_060e), // lsl r6, r1, #24 -- asr r7, r6, #8
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE12FFF1A_u32), // bx 0x00100000
            (0x08000010_u32, NOP),
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
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(1, 0), 0xff);
        assert_eq!(cpu.rf.get_register(2, 0), 0xff << 5);
        assert_eq!(cpu.rf.get_register(3, 0), 0xff >> 5);
        assert_eq!(cpu.rf.get_register(5, 0), 0xff >> 5);
        assert_eq!(cpu.rf.get_register(6, 0), 0xff000000);
        assert_eq!(cpu.rf.get_register(7, 0), 0xffff0000);
    }

    #[test]
    fn thumb_add_subtract_test() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x00100000_u32, 0x22fe_21ff), // mov r1, 0xff  --  mov r2, 0xfe
            (0x00100004_u32, 0x1a54_188b), // add r3, r1, r2 -- sub r4, r2, r1
            (0x00100008_u32, 0x1e4e_1c4d), // add r5, r1, 1 -- sub r6, r1, 1
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE12FFF1A_u32), // bx 0x00100000
            (0x08000010_u32, NOP),
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
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(1, 0), 0xff);
        assert_eq!(cpu.rf.get_register(2, 0), 0xfe);
        assert_eq!(cpu.rf.get_register(3, 0), 0x1fd);
        assert_eq!(cpu.rf.get_register(4, 0), 0xffffffff);
        assert_eq!(cpu.rf.get_register(5, 0), 0x100);
        assert_eq!(cpu.rf.get_register(6, 0), 0xfe);
    }

    #[test]
    fn thumb_alu_1() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x00100000_u32, 0x21fe_20ff),
            (0x00100004_u32, 0x22fe_4001),
            (0x00100008_u32, 0x2003_4042),
            (0x0010000c_u32, 0x4083_23ff),
            (0x00100010_u32, 0x40c4_24ff),
            (0x00100014_u32, 0x4105_25ff),
            (0x00100018_u32, 0x4146_26ff),
            (0x0010001c_u32, 0x4187_27ff),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE12FFF1A_u32), // bx 0x00100000
            (0x08000010_u32, NOP),
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
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(0, 0), 3);
        assert_eq!(cpu.rf.get_register(1, 0), 0xff & 0xfe);
        assert_eq!(cpu.rf.get_register(2, 0), 0xff ^ 0xfe);
        assert_eq!(cpu.rf.get_register(3, 0), 0xff << 3);
        assert_eq!(cpu.rf.get_register(4, 0), 0xff >> 3);
        assert_eq!(cpu.rf.get_register(5, 0), 0xff >> 3);
        assert_eq!(cpu.rf.get_register(6, 0), 0xff + 3);
        assert_eq!(cpu.rf.get_register(7, 0), 0xff - 3 - 1);
    }

    #[test]
    fn thumb_alu_2() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x00100000_u32, 0x2003_2110),
            (0x00100004_u32, 0x2201_41c1),
            (0x00100008_u32, 0x428a_420a),
            (0x0010000c_u32, 0x4253_42ca),
            (0x00100010_u32, 0x4304_2408),
            (0x00100014_u32, 0x4365_25ff),
            (0x00100018_u32, 0x43e7_439e),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE12FFF1A_u32), // bx 0x00100000
            (0x08000010_u32, NOP),
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
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(0, 0), 3);
        assert_eq!(cpu.rf.get_register(1, 0), 2);
        assert_eq!(cpu.rf.get_register(2, 0), 1);
        assert_eq!(cpu.rf.get_register(3, 0), 0xffffffff);
        assert_eq!(cpu.rf.get_register(4, 0), 11);
        assert_eq!(cpu.rf.get_register(5, 0), 0xaf5);
        assert_eq!(cpu.rf.get_register(6, 0), 0);
        assert_eq!(cpu.rf.get_register(7, 0), !11_u32);
    }

    #[test]
    fn thumb_pc_relative_load() {
        let mut cpu = ARM7TDMI::new();

        let mut instructions = HashMap::from([
            (0x00100000_u32, 0b0100011011000000_0100100000000100), // LDR r0 [pc, 4] -- nop
            (0x00100014_u32, !0_u32),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE12FFF1A_u32), // bx 0x00100000
            (0x08000010_u32, NOP),
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..12 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(0, 0), !0);
    }

    #[test]
    fn thumb_load_store_reg_offset() {
        let mut cpu = ARM7TDMI::new();

        // 0 -> 0xaaaaaaaa
        // mov r0, 0
        // mov r1, 0x100
        // add r1, 0xff -> r1 == 0x1ff
        // mov r2, 0x100
        // str r1, [r0, r2] -> 0x100 == 0x1ff
        // add r2, 4
        // strb r1, [r0, r2] -> 0x104 == 0xff
        // ldr r3, [r0, r0] -> r3 == 0xaaaaaaaa
        // ldrb r4, [r0, r0] -> r4 == 0xaa

        let mut instructions = HashMap::from([
            (0x00000000_u32, 0xaaaaaaaa),
            (0x00100000_u32, 0x21ff_2000),
            (0x00100004_u32, 0x31ff_3101),
            (0x00100008_u32, 0x3201_22ff),
            (0x0010000c_u32, 0x3204_5081),
            (0x00100010_u32, 0x5803_5481),
            (0x00100014_u32, 0x0000_5c04),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE12FFF1A_u32), // bx 0x00100000
            (0x08000010_u32, NOP),
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..100 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(0, 0), 0);
        assert_eq!(cpu.rf.get_register(1, 0), 0x1ff);
        assert_eq!(cpu.rf.get_register(2, 0), 0x104);
        assert_eq!(*instructions.get(&0x100).unwrap_or(&0), 0x1ff);
        assert_eq!(*instructions.get(&0x104).unwrap_or(&0), !0);
        assert_eq!(cpu.rf.get_register(3, 0), 0xaaaaaaaa);
        assert_eq!(cpu.rf.get_register(4, 0), 0xaa);
    }

    #[test]
    fn thumb_load_store_imm_offset() {
        let mut cpu = ARM7TDMI::new();

        // 0 -> 0xaaaaaaaa
        // mov r0, #0
        // mov r1, #0xff
        // add r1, #0xff
        // str r1, [r0, #0x4]
        // strb r1, [r0, #0x8]
        // ldr r3, [r0, #0x0]
        // ldrb r4, [r0, #0x0]

        let mut instructions = HashMap::from([
            (0x00000000_u32, 0xaaaaaaaa),
            (0x00100000_u32, 0x21ff_2000),
            (0x00100004_u32, 0x6041_31ff),
            (0x00100008_u32, 0x6803_7201),
            (0x0010000c_u32, 0x0000_7804),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE12FFF1A_u32), // bx 0x00100000
            (0x08000010_u32, NOP),
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..100 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(0, 0), 0);
        assert_eq!(cpu.rf.get_register(1, 0), 0x1fe);
        assert_eq!(*instructions.get(&0x4).unwrap_or(&0), 0x1fe);
        assert_eq!(*instructions.get(&0x8).unwrap_or(&0), 0xfefefefe);
        assert_eq!(cpu.rf.get_register(3, 0), 0xaaaaaaaa);
        assert_eq!(cpu.rf.get_register(4, 0), 0xaa);
    }

    #[test]
    fn thumb_sp_relative_load_store() {
        let mut cpu = ARM7TDMI::new();

        // 0 -> 0xaaaaaaaa
        // r13 == 0
        //
        // mov r1, #0xff
        // ldr r0, [sp, #0]
        // str r1, [sp, #4]

        let mut instructions = HashMap::from([
            (0x00000000_u32, 0xaaaaaaaa),
            (0x00100000_u32, 0x9800_21ff),
            (0x00100004_u32, 0x0000_9101),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE3A0D000_u32),
            (0x08000010_u32, 0xE12FFF1A_u32), // bx 0x00100000
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..100 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(0, 0), 0xaaaaaaaa);
        assert_eq!(*instructions.get(&0x4).unwrap_or(&0), 0xff);
    }

    #[test]
    fn thumb_load_store_halfword() {
        let mut cpu = ARM7TDMI::new();

        // 0 -> 0x01234567
        //
        //  ldrh r1, [r0, #0]
        //  ldrh r2, [r0, #2]
        //  strh r1, [r0, #8]
        //  strh r2, [r0, #10]

        let mut instructions = HashMap::from([
            (0x00000000_u32, 0x01234567),
            (0x00100000_u32, 0x8842_8801),
            (0x00100004_u32, 0x8142_8101),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE3A0D000_u32),
            (0x08000010_u32, 0xE12FFF1A_u32), // bx 0x00100000
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..100 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(1, 0), 0x4567);
        assert_eq!(cpu.rf.get_register(2, 0), 0x0123);
        assert_eq!(*instructions.get(&0x8).unwrap_or(&0), 0x45674567);
    }

    #[test]
    fn thumb_load_store_sign_ext() {
        let mut cpu = ARM7TDMI::new();

        // 0 -> 0x0102a304
        // 4 -> 0x0123abcd
        //
        //  mov r1, 0xff
        //  add r1, 0xff
        //  mov r2, 8
        //  mov r4, 5
        //  strh r1, [r0, r2] -> [0x08] == 0x1fe
        //  ldsb r3, [r0, r4] -> r3 == 0xffffffab
        //  ldrh r6, [r0, r0] -> r6 == 0x0000a304
        //  ldsh r7, [r0, r0] -> r7 == 0xffffa304

        let mut instructions = HashMap::from([
            (0x00000000_u32, 0x0102a304),
            (0x00000004_u32, 0x0123abcd),
            (0x00100000_u32, 0x31ff_21ff),
            (0x00100004_u32, 0x2405_2208),
            (0x00100008_u32, 0x5703_5281),
            (0x0010000c_u32, 0x5e07_5a06),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE3A0D000_u32),
            (0x08000010_u32, 0xE12FFF1A_u32), // bx 0x00100000
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..100 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(*instructions.get(&0x8).unwrap_or(&0) & 0xfff, 0x1fe);
        assert_eq!(cpu.rf.get_register(3, 0), 0xffffffab);
        assert_eq!(cpu.rf.get_register(6, 0), 0x0000a304);
        assert_eq!(cpu.rf.get_register(7, 0), 0xffffa304);
    }

    #[test]
    fn thumb_branch() {
        let mut cpu = ARM7TDMI::new();

        //      mov r1, #1
        //      b label1
        //
        //      mov r1, #10
        // label1:
        //      mov r2, #20
        //      cmp r2, #20
        //      beq label2
        //
        //      mov r3, #30
        //
        // label2:
        //      add r3, #33
        //      cmp r2, #20
        //      bne label3
        //      add r4, #40
        //
        //  label3:
        //      b label3
        //

        let mut instructions = HashMap::from([
            (0x00100000_u32, 0xe000_2101),
            (0x00100004_u32, 0x2214_210a),
            (0x00100008_u32, 0xd000_2a14),
            (0x0010000c_u32, 0x3321_231e),
            (0x00100010_u32, 0xd100_2a14),
            (0x00100014_u32, 0xe7fe_3428),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE3A0D000_u32),
            (0x08000010_u32, 0xE12FFF1A_u32), // bx 0x00100000
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..100 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(1, 0), 1);
        assert_eq!(cpu.rf.get_register(2, 0), 20);
        assert_eq!(cpu.rf.get_register(3, 0), 33);
        assert_eq!(cpu.rf.get_register(4, 0), 40);
    }

    #[test]
    fn thumb_swi() {
        let mut cpu = ARM7TDMI::new();

        // 0x08 -> mov r9, 9
        // swi 0

        let mut instructions = HashMap::from([
            (0x00000008_u32, 0xE3A09009_u32),
            (0x00100000_u32, 0x0000_df00),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE3A0D000_u32),
            (0x08000010_u32, 0xE12FFF1A_u32), // bx 0x00100000
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..100 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(9, 0), 9);
    }

    #[test]
    fn thumb_push_pop() {
        let mut cpu = ARM7TDMI::new();

        // main_thumb:
        //  mov r1, #10
        //  mov r2, #20
        //  push {r1, r2}
        //  mov r3, #30
        //  mov r4, #40
        //  pop {r3, r4}
        //

        let mut instructions = HashMap::from([
            (0x00100000_u32, 0x2214_210a),
            (0x00100004_u32, 0x231e_b406),
            (0x00100008_u32, 0xbc18_2428),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE3A0D000_u32),
            (0x08000010_u32, 0xE12FFF1A_u32), // bx 0x00100000
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..100 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(3, 0), 10);
        assert_eq!(cpu.rf.get_register(4, 0), 20);
    }

    #[test]
    fn ldm_stm_thumb() {
        let mut cpu = ARM7TDMI::new();

        // main_thumb:
        //  mov r1, #10
        //  mov r2, #20
        //  stmia r3!, {r1, r2}
        //  mov r3, #30
        //  mov r4, #40
        //  ldmia r5!, {r3, r4}
        //

        let mut instructions = HashMap::from([
            (0x00100000_u32, 0x2214_210a),
            (0x00100004_u32, 0x231e_c306),
            (0x00100008_u32, 0xcd18_2428),
            (0x08000000_u32, NOP),
            (0x08000004_u32, 0xE3A0A601_u32),
            (0x08000008_u32, 0xE28AA001_u32),
            (0x0800000c_u32, 0xE3A0D000_u32),
            (0x08000010_u32, 0xE12FFF1A_u32), // bx 0x00100000
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..100 {
            let req = cpu.step(response);
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(*instructions.get(&0).unwrap_or(&0), 10);
        assert_eq!(*instructions.get(&4).unwrap_or(&0), 20);
        assert_eq!(cpu.rf.get_register(5, 0), 4);
    }

    #[test]
    fn bl_thumb() {
        let mut cpu = ARM7TDMI::new();

        // main_thumb:
        //
        //	mov r0, #0
        //  bl function
        // end:
        //  b end
        //
        // function:
        //  mov r0, #10
        //  mov r15, r14

        let mut instructions = HashMap::from([
            (0x00000000_u32, 0xf000_2000),
            (0x00000004_u32, 0xe7fe_f801),
            (0x00000008_u32, 0x46f7_200a),
            (0x0000000c_u32, 0x0000_0000),
            (0x08000000_u32, NOP),
            (0x08000000_u32, 0xE28AA001_u32),
            (0x08000008_u32, 0xE12FFF1A_u32), // bx 0
        ]);

        let mut response = MemoryResponse {
            data: NOP,
            n_wait: BusSignal::HIGH,
        };

        for _ in 0..100 {
            let req = cpu.step(response);
            println!(
                "{:#06X} -> R15 is {:#010X}",
                cpu.arm_current_execute,
                cpu.rf.get_register(15, 0)
            );
            if req.nr_w == BusSignal::LOW {
                response.data = *instructions
                    .get(&(req.address & 0xFFFFFFFC))
                    .unwrap_or(&NOP_THUMB);
            } else {
                instructions.insert(req.address, req.data);
            }
        }

        assert_eq!(cpu.rf.get_register(0, 0), 10);
    }
}
