use crate::arm7_tdmi::instruction::barrel_shifter;
use crate::arm7_tdmi::instruction::ArmAluOpcode;
use crate::arm7_tdmi::{InstructionStep, ARM7TDMI};
use crate::bus::{BusCycle, MemoryRequest, MemoryResponse, TransferSize};
use crate::common::BitOperation;

impl ARM7TDMI {
    pub fn thumb_move_shifter_register(&mut self) {
        todo!()
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
    pub fn thumb_alu(&self) {
        todo!()
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

    pub fn thumb_pc_relative_load(&self) {
        todo!()
    }
    pub fn thumb_load_store_reg_offset(&self) {
        todo!()
    }
    pub fn thumb_load_store_sign_ext(&self) {
        todo!()
    }
    pub fn thumb_load_store_imm_offset(&self) {
        todo!()
    }
    pub fn thumb_load_store_halfword(&self) {
        todo!()
    }
    pub fn thumb_sp_relative_load_store(&self) {
        todo!()
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
