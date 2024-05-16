use crate::arm7_tdmi::instruction::barrel_shifter;
use crate::arm7_tdmi::instruction::ArmAluOpcode;
use crate::arm7_tdmi::register_file::ConditionCodeFlag;
use crate::arm7_tdmi::{InstructionStep, ARM7TDMI};
use crate::bus::{BusCycle, BusSignal, MemoryRequest, MemoryResponse, TransferSize};
use crate::common::BitOperation;

impl ARM7TDMI {
    pub fn thumb_move_shifter_register(&mut self) {
        let opcode = self.arm_current_execute.get_range(12, 11);
        let offset = self.arm_current_execute.get_range(10, 6);
        let rs = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);
        let current_c = self.rf.is_flag_set(&ConditionCodeFlag::C);

        let (result, mut c_out, there_is_shift) = barrel_shifter(
            self.rf.get_register(rs, 4),
            opcode,
            offset,
            current_c,
            false,
        );

        self.rf.write_register(rd, result);
        if !there_is_shift {
            c_out = current_c
        }

        self.update_flags(
            result,
            ArmAluOpcode::MOV,
            rd,
            false,
            c_out,
            self.rf.is_flag_set(&ConditionCodeFlag::V),
        );
    }

    pub fn thumb_add_subtract(&mut self) {
        let opcode = self.arm_current_execute.get_range(10, 9);
        let rn = self.arm_current_execute.get_range(8, 6);
        let rs = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);
        let (result, c_flag, v_flag): (u32, bool, bool);

        if opcode == 0 {
            (result, c_flag, v_flag) = self.alu_operation(
                self.rf.get_register(rs, 0),
                self.rf.get_register(rn, 0),
                ArmAluOpcode::ADD,
            );
        } else if opcode == 1 {
            (result, c_flag, v_flag) = self.alu_operation(
                self.rf.get_register(rs, 0),
                self.rf.get_register(rn, 0),
                ArmAluOpcode::SUB,
            );
        } else if opcode == 2 {
            (result, c_flag, v_flag) =
                self.alu_operation(self.rf.get_register(rs, 0), rn, ArmAluOpcode::ADD);
        } else {
            (result, c_flag, v_flag) =
                self.alu_operation(self.rf.get_register(rs, 0), rn, ArmAluOpcode::SUB);
        }

        self.rf.write_register(rd, result);
        // No matter if it's ADD or SUB, it is still and arithmetic operation
        self.update_flags(result, ArmAluOpcode::ADD, rd, c_flag, false, v_flag);
    }

    pub fn thumb_alu_immediate(&mut self) {
        let opcode = self.arm_current_execute.get_range(12, 11);
        let rd = self.arm_current_execute.get_range(10, 8);
        let nn = self.arm_current_execute.get_range(7, 0);
        let (result, c_flag, v_flag): (u32, bool, bool);

        if opcode == 0 {
            (result, c_flag, v_flag) = self.alu_operation(0, nn, ArmAluOpcode::MOV);
            self.rf.write_register(rd, result);
            self.update_flags(result, ArmAluOpcode::MOV, rd, c_flag, false, v_flag);
        } else if opcode == 1 {
            (result, c_flag, v_flag) =
                self.alu_operation(self.rf.get_register(rd, 0), nn, ArmAluOpcode::CMP);
            self.update_flags(result, ArmAluOpcode::CMP, rd, c_flag, false, v_flag);
        } else if opcode == 2 {
            (result, c_flag, v_flag) =
                self.alu_operation(self.rf.get_register(rd, 0), nn, ArmAluOpcode::ADD);
            self.rf.write_register(rd, result);
            self.update_flags(result, ArmAluOpcode::ADD, rd, c_flag, false, v_flag);
        } else {
            (result, c_flag, v_flag) =
                self.alu_operation(self.rf.get_register(rd, 0), nn, ArmAluOpcode::SUB);
            self.rf.write_register(rd, result);
            self.update_flags(result, ArmAluOpcode::SUB, rd, c_flag, false, v_flag);
        }
    }
    pub fn thumb_alu(&mut self, req: &mut MemoryRequest) {
        let opcode = self.arm_current_execute.get_range(9, 6);
        let rs = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);
        let alu_result;
        let there_is_shift;
        let mut carry_shifter = self.rf.is_flag_set(&ConditionCodeFlag::C);
        let mut c_flag = false;
        let mut v_flag = false;
        let mut instruction_type = ArmAluOpcode::MOV;

        if self.instruction_step == InstructionStep::STEP0 {
            let mut ops = self.rf.get_register(rs, 0);
            let mut opd = self.rf.get_register(rd, 0);

            match opcode {
                0x0 => {
                    instruction_type = ArmAluOpcode::AND;
                    (alu_result, c_flag, v_flag) = self.alu_operation(ops, opd, instruction_type);
                }
                0x1 => {
                    instruction_type = ArmAluOpcode::EOR;
                    (alu_result, c_flag, v_flag) = self.alu_operation(ops, opd, instruction_type);
                }
                0x2 => {
                    self.instruction_step = InstructionStep::STEP1;
                    self.instruction_counter_step = 1;
                    req.bus_cycle = BusCycle::INTERNAL;
                    self.data_is_fetch = false;
                    instruction_type = ArmAluOpcode::MOV;
                    ops = ops & 0xff;
                    (alu_result, carry_shifter, there_is_shift) = barrel_shifter(
                        opd,
                        0,
                        ops,
                        self.rf.is_flag_set(&ConditionCodeFlag::C),
                        true,
                    );
                    if !there_is_shift {
                        carry_shifter = self.rf.is_flag_set(&ConditionCodeFlag::C);
                    }
                }
                0x3 => {
                    self.instruction_step = InstructionStep::STEP1;
                    self.instruction_counter_step = 1;
                    req.bus_cycle = BusCycle::INTERNAL;
                    self.data_is_fetch = false;
                    instruction_type = ArmAluOpcode::MOV;
                    ops = ops & 0xff;
                    (alu_result, carry_shifter, there_is_shift) = barrel_shifter(
                        opd,
                        1,
                        ops,
                        self.rf.is_flag_set(&ConditionCodeFlag::C),
                        true,
                    );
                    if !there_is_shift {
                        carry_shifter = self.rf.is_flag_set(&ConditionCodeFlag::C);
                    }
                }
                0x4 => {
                    self.instruction_step = InstructionStep::STEP1;
                    self.instruction_counter_step = 1;
                    req.bus_cycle = BusCycle::INTERNAL;
                    self.data_is_fetch = false;
                    instruction_type = ArmAluOpcode::MOV;
                    ops = ops & 0xff;
                    (alu_result, carry_shifter, there_is_shift) = barrel_shifter(
                        opd,
                        2,
                        ops,
                        self.rf.is_flag_set(&ConditionCodeFlag::C),
                        true,
                    );
                    if !there_is_shift {
                        carry_shifter = self.rf.is_flag_set(&ConditionCodeFlag::C);
                    }
                }
                0x5 => {
                    instruction_type = ArmAluOpcode::ADC;
                    (alu_result, c_flag, v_flag) = self.alu_operation(opd, ops, instruction_type);
                }
                0x6 => {
                    instruction_type = ArmAluOpcode::SBC;
                    (alu_result, c_flag, v_flag) = self.alu_operation(opd, ops, instruction_type);
                }
                0x7 => {
                    self.instruction_step = InstructionStep::STEP1;
                    self.instruction_counter_step = 1;
                    req.bus_cycle = BusCycle::INTERNAL;
                    self.data_is_fetch = false;
                    instruction_type = ArmAluOpcode::MOV;
                    ops = ops & 0xff;

                    (alu_result, carry_shifter, there_is_shift) = barrel_shifter(
                        opd,
                        3,
                        ops,
                        self.rf.is_flag_set(&ConditionCodeFlag::C),
                        true,
                    );
                    if !there_is_shift {
                        carry_shifter = self.rf.is_flag_set(&ConditionCodeFlag::C);
                    }
                }
                0x8 => {
                    instruction_type = ArmAluOpcode::TST;
                    (alu_result, c_flag, v_flag) = self.alu_operation(opd, ops, instruction_type);
                }
                0x9 => {
                    opd = 0;
                    instruction_type = ArmAluOpcode::RSB;
                    (alu_result, c_flag, v_flag) = self.alu_operation(ops, opd, instruction_type);
                }
                0xa => {
                    instruction_type = ArmAluOpcode::CMP;
                    (alu_result, c_flag, v_flag) = self.alu_operation(opd, ops, instruction_type);
                }
                0xb => {
                    instruction_type = ArmAluOpcode::CMN;
                    (alu_result, c_flag, v_flag) = self.alu_operation(opd, ops, instruction_type);
                }
                0xc => {
                    instruction_type = ArmAluOpcode::ORR;
                    (alu_result, c_flag, v_flag) = self.alu_operation(opd, ops, instruction_type);
                }
                0xd => {
                    self.instruction_step = InstructionStep::STEP1;
                    req.bus_cycle = BusCycle::INTERNAL;
                    self.data_is_fetch = false;
                    self.instruction_counter_step = 4 - (opd.leading_zeros() >> 3);
                    alu_result = ((ops as u64) * (opd as u64)) as u32
                }
                0xe => {
                    instruction_type = ArmAluOpcode::BIC;
                    (alu_result, c_flag, v_flag) = self.alu_operation(opd, ops, instruction_type);
                }
                0xf => {
                    instruction_type = ArmAluOpcode::MNV;
                    (alu_result, c_flag, v_flag) = self.alu_operation(opd, ops, instruction_type);
                }
                _ => {
                    panic!("Wrong opcode for instruction ALU THUMB");
                }
            }

            if ![0x8, 0xa, 0xb].contains(&opcode) {
                self.rf.write_register(rd, alu_result);
            }

            if opcode != 0xd {
                self.update_flags(
                    alu_result,
                    instruction_type,
                    rd,
                    c_flag,
                    carry_shifter,
                    v_flag,
                )
            }
        } else if self.instruction_step == InstructionStep::STEP1 {
            self.instruction_counter_step -= 1;
            if self.instruction_counter_step == 0 {
                self.instruction_step = InstructionStep::STEP0;
            } else {
                self.data_is_fetch = false;
                req.bus_cycle = BusCycle::INTERNAL;
            }
        } else {
            panic!("Wrong step for instruction ALU THUMB");
        }
    }

    pub fn thumb_hi_register_bx(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(9, 8);
        let msbd = self.arm_current_execute.get_range(7, 7);
        let msbs = self.arm_current_execute.get_range(6, 6);
        let rs = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        let full_rd = if msbd == 1 { rd | 0x8 } else { rd };
        let full_rs = if msbs == 1 { rs | 0x8 } else { rs };

        let bx_address = self.rf.get_register(full_rs, 4);

        // TODO: handle case of destination being r15
        // THIS INSTRUCTION IS NOT COMPLETE
        if self.instruction_step == InstructionStep::STEP0 {
            if opcode == 0 {
                let op1 = self.rf.get_register(full_rd, 4);
                let op2 = self.rf.get_register(full_rs, 4);
                let (result, _, _) = self.alu_operation(op1, op2, ArmAluOpcode::ADD);
                self.rf.write_register(full_rd, result);
                if full_rd == 15 {
                    self.rf
                        .write_register(full_rd, self.rf.get_register(15, 0) & !2);
                    self.arm_instruction_queue.clear();
                    req.bus_cycle = BusCycle::NONSEQUENTIAL;
                    self.data_is_fetch = false;
                    self.instruction_step = InstructionStep::STEP1;
                }
            } else if opcode == 1 {
                let op1 = self.rf.get_register(full_rd, 4);
                let op2 = self.rf.get_register(full_rs, 4);
                let (result, c_flag, v_flag) = self.alu_operation(op1, op2, ArmAluOpcode::CMP);
                self.rf.write_register(full_rd, result);
                self.update_flags(result, ArmAluOpcode::CMP, full_rd, c_flag, true, v_flag);
            } else if opcode == 2 {
                let op1 = self.rf.get_register(full_rd, 4);
                let op2 = self.rf.get_register(full_rs, 4);
                let (result, _, _) = self.alu_operation(op1, op2, ArmAluOpcode::MOV);
                self.rf.write_register(full_rd, result);
                if full_rd == 15 {
                    self.rf
                        .write_register(full_rd, self.rf.get_register(15, 0) & !2);
                    self.arm_instruction_queue.clear();
                    req.bus_cycle = BusCycle::NONSEQUENTIAL;
                    self.data_is_fetch = false;
                    self.instruction_step = InstructionStep::STEP1;
                }
            } else {
                self.arm_instruction_queue.clear();
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP3;
            }
        } else if self.instruction_step == InstructionStep::STEP1 {
            req.mas = TransferSize::WORD;
            req.address = self.rf.get_register(15, 0);
            req.bus_cycle = BusCycle::SEQUENTIAL;
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP2;
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.mas = TransferSize::WORD;
            req.address = self.rf.get_register(15, 4);
            self.arm_instruction_queue.push_back(rsp.data);
            self.rf
                .write_register(15, (self.rf.get_register(15, 0)).wrapping_sub(4));
            let _ = self.rf.write_cpsr(self.rf.get_cpsr().clear_bit(5));
        } else if self.instruction_step == InstructionStep::STEP3 {
            if bx_address.is_bit_set(0) {
                req.mas = TransferSize::HALFWORD;
            } else {
                req.mas = TransferSize::WORD;
            }
            req.address = bx_address;
            req.bus_cycle = BusCycle::SEQUENTIAL;
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP4;
        } else if self.instruction_step == InstructionStep::STEP4 {
            if msbd == 1 {
                self.rf.write_register(14, self.rf.get_register(15, 0))
            }
            if bx_address.is_bit_set(0) {
                if self.last_used_address.is_bit_clear(1) {
                    self.arm_instruction_queue
                        .push_back(rsp.data.get_range(15, 0));
                } else {
                    self.arm_instruction_queue
                        .push_back(rsp.data.get_range(31, 16));
                }
                req.mas = TransferSize::HALFWORD;
                req.address = bx_address.wrapping_add(2);
                self.rf.write_register(15, bx_address.wrapping_sub(2));
                let _ = self.rf.write_cpsr(self.rf.get_cpsr().set_bit(5));
            } else {
                req.mas = TransferSize::WORD;
                req.address = bx_address.wrapping_add(4);
                self.arm_instruction_queue.push_back(rsp.data);
                self.rf.write_register(15, bx_address.wrapping_sub(4));
                let _ = self.rf.write_cpsr(self.rf.get_cpsr().clear_bit(5));
            }
            req.bus_cycle = BusCycle::SEQUENTIAL;
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for THUMB HI REGISTER BX");
        }
    }

    pub fn thumb_pc_relative_load(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let rd = self.arm_current_execute.get_range(10, 8);
        let nn = self.arm_current_execute.get_range(7, 0);

        if self.instruction_step == InstructionStep::STEP0 {
            req.bus_cycle = BusCycle::NONSEQUENTIAL;
            self.instruction_step = InstructionStep::STEP1;
        } else if self.instruction_step == InstructionStep::STEP1 {
            self.data_is_fetch = false;
            req.bus_cycle = BusCycle::INTERNAL;
            req.mas = TransferSize::WORD;
            req.address = (self.rf.get_register(15, 4) & !2).wrapping_add(nn << 2);
            self.instruction_step = InstructionStep::STEP2;
        } else if self.instruction_step == InstructionStep::STEP2 {
            let mut data_to_write = rsp.data;
            let offset = self.last_used_address % 4;
            data_to_write = data_to_write.rotate_right(offset * 8);

            self.rf.write_register(rd, data_to_write);
            self.data_is_fetch = false;

            req.bus_cycle = BusCycle::SEQUENTIAL;
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong instruction step for THUMB PC RELATIVE LOAD");
        }
    }

    pub fn thumb_load_store_reg_offset(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 10);
        let ro = self.arm_current_execute.get_range(8, 6);
        let rb = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        // STR
        if opcode < 2 {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                req.data = self.rf.get_register(rd, 0);
                req.address = self
                    .rf
                    .get_register(rb, 0)
                    .wrapping_add(self.rf.get_register(ro, 0));

                // If only one byte is to be moved, copy the byte over all the 32 lines of the bus.
                if opcode == 1 {
                    let byte = req.data & 0xff;
                    req.data = byte | (byte << 8) | (byte << 16) | (byte << 24);
                    req.mas = TransferSize::BYTE;
                } else {
                    req.mas = TransferSize::WORD;
                }

                req.nr_w = BusSignal::HIGH;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong instruction step for instruction THUMB STR");
            }
        // LDR
        } else {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                self.data_is_fetch = false;
                req.bus_cycle = BusCycle::INTERNAL;
                req.address = self
                    .rf
                    .get_register(rb, 0)
                    .wrapping_add(self.rf.get_register(ro, 0));
                if opcode == 3 {
                    req.mas = TransferSize::BYTE;
                } else {
                    req.mas = TransferSize::WORD;
                }
                self.instruction_step = InstructionStep::STEP2;
            } else if self.instruction_step == InstructionStep::STEP2 {
                let mut data_to_write = rsp.data;
                let offset = self.last_used_address % 4;

                if opcode == 3 {
                    data_to_write = data_to_write.get_range(offset * 8 + 7, offset * 8);
                } else {
                    data_to_write = data_to_write.rotate_right(offset * 8);
                }

                self.rf.write_register(rd, data_to_write);
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong instruction step for instruction THUMB STR");
            }
        }
    }

    pub fn thumb_load_store_sign_ext(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 10);
        let ro = self.arm_current_execute.get_range(8, 6);
        let rb = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);
        let offset = self.rf.get_register(ro, 0);

        // STRH
        if opcode == 0 {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                req.mas = TransferSize::HALFWORD;
                self.data_is_fetch = false;
                req.data = self.rf.get_register(rd, 12);
                req.data = (req.data & 0xffff) | (req.data << 16);
                req.address = self.rf.get_register(rb, 0).wrapping_add(offset);
                req.nr_w = BusSignal::HIGH;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong step for instruction STRH THUMB")
            }

        // LDSB / LDRH / LDSH
        } else {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                self.data_is_fetch = false;
                req.bus_cycle = BusCycle::INTERNAL;
                req.address = self.rf.get_register(rb, 0).wrapping_add(offset);
                req.mas = TransferSize::HALFWORD;
                self.instruction_step = InstructionStep::STEP2;
            } else if self.instruction_step == InstructionStep::STEP2 {
                let mut data_to_write = rsp.data;
                let offset = self.last_used_address % 4;

                if opcode == 1 {
                    data_to_write = data_to_write.get_range(offset * 8 + 7, offset * 8);
                    data_to_write = ((data_to_write as i8) as i32) as u32;
                } else {
                    data_to_write = if offset < 2 {
                        data_to_write.get_range(15, 0)
                    } else {
                        data_to_write.get_range(31, 16)
                    };

                    if offset % 2 == 1 {
                        data_to_write = data_to_write.rotate_right(8);
                    }

                    if opcode == 3 {
                        data_to_write = ((data_to_write as i16) as i32) as u32;
                    }
                }

                // Update the destination register
                self.rf.write_register(rd, data_to_write);
                self.data_is_fetch = false;

                req.bus_cycle = BusCycle::SEQUENTIAL;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong step for instruction STRH THUMB")
            }
        }
    }

    pub fn thumb_load_store_imm_offset(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(12, 11);
        let offset = if opcode > 1 {
            self.arm_current_execute.get_range(10, 6)
        } else {
            self.arm_current_execute.get_range(10, 6) * 4
        };
        let rb = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        // STR
        if opcode & 1 == 0 {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                req.data = self.rf.get_register(rd, 0);
                req.address = self.rf.get_register(rb, 0).wrapping_add(offset);

                // If only one byte is to be moved, copy the byte over all the 32 lines of the bus.
                if opcode == 2 {
                    let byte = req.data & 0xff;
                    req.data = byte | (byte << 8) | (byte << 16) | (byte << 24);
                    req.mas = TransferSize::BYTE;
                } else {
                    req.mas = TransferSize::WORD;
                }

                req.nr_w = BusSignal::HIGH;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong instruction step for instruction THUMB STR");
            }
        // LDR
        } else {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                self.data_is_fetch = false;
                req.bus_cycle = BusCycle::INTERNAL;
                req.address = self.rf.get_register(rb, 0).wrapping_add(offset);
                if opcode == 3 {
                    req.mas = TransferSize::BYTE;
                } else {
                    req.mas = TransferSize::WORD;
                }
                self.instruction_step = InstructionStep::STEP2;
            } else if self.instruction_step == InstructionStep::STEP2 {
                let mut data_to_write = rsp.data;
                let offset = self.last_used_address % 4;

                if opcode == 3 {
                    data_to_write = data_to_write.get_range(offset * 8 + 7, offset * 8);
                } else {
                    data_to_write = data_to_write.rotate_right(offset * 8);
                }

                self.rf.write_register(rd, data_to_write);
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong instruction step for instruction THUMB STR");
            }
        }
    }

    pub fn thumb_load_store_halfword(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 11);
        let nn = self.arm_current_execute.get_range(10, 6) << 1;
        let rb = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        // STRH
        if opcode == 0 {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                req.mas = TransferSize::HALFWORD;
                self.data_is_fetch = false;
                req.data = self.rf.get_register(rd, 12);
                req.data = (req.data & 0xffff) | (req.data << 16);
                req.address = self.rf.get_register(rb, 0).wrapping_add(nn);
                req.nr_w = BusSignal::HIGH;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong step for instruction STRH THUMB")
            }
        // LDRH
        } else {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                self.data_is_fetch = false;
                req.bus_cycle = BusCycle::INTERNAL;
                req.address = self.rf.get_register(rb, 0).wrapping_add(nn);
                req.mas = TransferSize::HALFWORD;
                self.instruction_step = InstructionStep::STEP2;
            } else if self.instruction_step == InstructionStep::STEP2 {
                let mut data_to_write = rsp.data;
                let offset = self.last_used_address % 4;

                data_to_write = if offset < 2 {
                    data_to_write.get_range(15, 0)
                } else {
                    data_to_write.get_range(31, 16)
                };

                if offset % 2 == 1 {
                    data_to_write = data_to_write.rotate_right(8);
                }

                // Update the destination register
                self.rf.write_register(rd, data_to_write);
                self.data_is_fetch = false;

                req.bus_cycle = BusCycle::SEQUENTIAL;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong step for instruction STRH THUMB")
            }
        }
    }

    pub fn thumb_sp_relative_load_store(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 11);
        let rd = self.arm_current_execute.get_range(10, 8);
        let nn = self.arm_current_execute.get_range(7, 0);

        // STR
        if opcode == 0 {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                req.data = self.rf.get_register(rd, 0);
                req.address = self.rf.get_register(13, 0).wrapping_add(nn << 2);
                req.mas = TransferSize::WORD;
                req.nr_w = BusSignal::HIGH;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong instruction step for instruction THUMB STR");
            }
        // LDR
        } else {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                self.data_is_fetch = false;
                req.bus_cycle = BusCycle::INTERNAL;
                req.address = self.rf.get_register(13, 0).wrapping_add(nn << 2);
                req.mas = TransferSize::WORD;
                self.instruction_step = InstructionStep::STEP2;
            } else if self.instruction_step == InstructionStep::STEP2 {
                let mut data_to_write = rsp.data;
                let offset = self.last_used_address % 4;
                data_to_write = data_to_write.rotate_right(offset * 8);
                self.rf.write_register(rd, data_to_write);
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong instruction step for instruction THUMB STR");
            }
        }
    }

    pub fn thumb_load_address(&mut self) {
        let opcode = self.arm_current_execute.get_range(11, 11);
        let rd = self.arm_current_execute.get_range(10, 8);
        let nn = self.arm_current_execute.get_range(7, 0) << 2;

        if opcode == 0 {
            self.rf
                .write_register(rd, (self.rf.get_register(15, 4) & !2).wrapping_add(nn));
        } else {
            self.rf
                .write_register(rd, self.rf.get_register(13, 0).wrapping_add(nn));
        }
    }

    pub fn thumb_add_offset_to_sp(&mut self) {
        let opcode = self.arm_current_execute.get_range(7, 7);
        let nn = self.arm_current_execute.get_range(6, 0) << 2;

        if opcode == 0 {
            self.rf
                .write_register(13, self.rf.get_register(13, 0).wrapping_add(nn));
        } else {
            self.rf
                .write_register(13, self.rf.get_register(13, 0).wrapping_add(nn));
        }
    }

    pub fn thumb_push_pop_register(&self) {
        todo!()
    }
    pub fn thumb_multiple_load_store(&self) {
        todo!()
    }
    pub fn thumb_conditional_branch(&self) {
        todo!()
    }
    pub fn thumb_software_interrupt(&self) {
        todo!()
    }
    pub fn thumb_unconditional_branch(&self) {
        todo!()
    }
    pub fn thumb_long_branch_with_link(&self) {
        todo!()
    }
}
