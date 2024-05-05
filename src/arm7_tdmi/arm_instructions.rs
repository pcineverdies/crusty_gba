use crate::arm7_tdmi::instruction::barrel_shifter;
use crate::arm7_tdmi::instruction::ArmAluOpcode;
use crate::arm7_tdmi::register_file::ConditionCodeFlag;
use crate::arm7_tdmi::OperatingMode;
use crate::arm7_tdmi::{InstructionStep, ARM7TDMI};
use crate::bus::{BusCycle, BusSignal, MemoryRequest, MemoryResponse, TransferSize};
use crate::common::BitOperation;

impl ARM7TDMI {
    /// arm7_tdmi::arm_data_processing
    ///
    /// function to handle all the data processing instructions (MOV, ADD, AND...)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    pub fn arm_data_processing(&mut self, req: &mut MemoryRequest) {
        // Destination address
        let rd = self.arm_current_execute.get_range(15, 12);

        if self.instruction_step == InstructionStep::STEP0 {
            // === Decode instruction ===

            // Opcode of the alu instruction
            let opcode = ArmAluOpcode::from_value(self.arm_current_execute.get_range(24, 21));
            // 1 if flags should be updated
            let s_flag = self.arm_current_execute.get_range(20, 20);
            // 1 if the operand to use is an immediate encoded in the msbs of the instruction
            let i_flag = self.arm_current_execute.get_range(25, 25);
            // Condition to be checked, otherwise instruction is skipped
            let condition = self.arm_current_execute.get_range(31, 28);
            // First operand
            let rn = self.arm_current_execute.get_range(19, 16);
            // Shift amount for register
            let mut shift_amount = self.arm_current_execute.get_range(11, 7);
            // Shift register
            let rs = self.arm_current_execute.get_range(11, 8);
            // What kind of shift
            let shift_type = self.arm_current_execute.get_range(6, 5);
            // Is shift done with a register or with an immediate
            let r_flag = self.arm_current_execute.get_range(4, 4);
            // Second operand
            let rm = self.arm_current_execute.get_range(3, 0);
            // Shift amount for immediate
            let is = self.arm_current_execute.get_range(11, 8);
            // Immediate value
            let nn = self.arm_current_execute.get_range(7, 0);

            let mut carry_shifter = self.rf.is_flag_set(&ConditionCodeFlag::C);
            let mut operand1 = self.rf.get_register(rn, 8);
            let mut operand2 = self.rf.get_register(rm, 8);
            let mut there_is_shift = false;

            if !self.rf.check_condition_code(condition) {
                return;
            }

            // operand1 is rn, operand 2 is nn << (2 * is)
            if i_flag == 1 {
                operand2 = nn.rotate_right(is * 2);

            // operand 1 is rn, operand 2 can be either `rm OP rs` or `rm op imm`
            } else {
                if r_flag == 1 {
                    if rs == 15 {
                        panic!("Cannot use r15 as rs register in ALU operations");
                    }
                    shift_amount = self.rf.get_register(rs, 0).get_range(7, 0);

                    // if rn == 15 or rm == 15, operands should be incremented
                    operand1 = self.rf.get_register(rn, 12);
                    operand2 = self.rf.get_register(rm, 12);
                }

                (operand2, carry_shifter, there_is_shift) = barrel_shifter(
                    operand2,
                    shift_type,
                    shift_amount,
                    self.rf.is_flag_set(&ConditionCodeFlag::C),
                );
            }

            // Get result from alu, and next value of carry and overflow flag in case of arithmetic
            // operations
            let (next_to_write, c_output, v_output) =
                self.alu_operation(operand1, operand2, opcode);

            // Write the result back for all the instructions which are not test
            if !ArmAluOpcode::is_test_opcode(opcode) {
                self.rf.write_register(rd, next_to_write);
            }

            // Update flags if the instruction is a test one or if s_flag is set
            if ArmAluOpcode::is_test_opcode(opcode) || s_flag == 1 {
                self.update_flags(next_to_write, opcode, rd, c_output, carry_shifter, v_output);
            }

            // If there is a shift, one extra cycle
            if there_is_shift {
                req.bus_cycle = BusCycle::INTERNAL;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP1;

            // If rd == 15, 2 more extra cycles to refill the pipeline
            } else if rd == 15 {
                self.arm_instruction_queue.clear();
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP2;
            }
        } else if self.instruction_step == InstructionStep::STEP1 {
            // If rd == 15, 2 more extra cycles to refill the pipeline
            if rd == 15 {
                self.arm_instruction_queue.clear();
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP2;
            } else {
                self.instruction_step = InstructionStep::STEP0;
            }
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.address = self.rf.get_register(15, 0);
            self.instruction_step = InstructionStep::STEP3;
        } else if self.instruction_step == InstructionStep::STEP3 {
            req.address = self.rf.get_register(15, 4);
            self.rf
                .write_register(15, self.rf.get_register(15, 0).wrapping_sub(4));
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for instructin type ARM_DATA_PROCESSING");
        }
    }

    /// arm7_tdmi::arm_branch_and_exchange
    ///
    /// TBD
    pub fn arm_branch_and_exchange(&mut self, _req: &mut MemoryRequest) {
        let condition = self.arm_current_execute.get_range(31, 28);

        if self.instruction_step == InstructionStep::STEP0 {
            if !self.rf.check_condition_code(condition) {
                return;
            }

            todo!("Switching to thumb mode behaviour");
        } else if self.instruction_step == InstructionStep::STEP1 {
        } else if self.instruction_step == InstructionStep::STEP2 {
        } else {
            panic!("Wrong step for instructin type ARM_BRANCH_AND_EXCHANGE");
        }
    }

    /// arm7_tdmi::arm_branch
    ///
    /// Function to handle all the branch instructions
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    pub fn arm_branch(&mut self, req: &mut MemoryRequest) {
        let condition = self.arm_current_execute.get_range(31, 28);
        let opcode = self.arm_current_execute.get_range(24, 24);
        let mut nn = self.arm_current_execute.get_range(23, 0);
        let current_pc = self.rf.get_register(15, 0);

        if !self.rf.check_condition_code(condition) {
            return;
        }

        if self.instruction_step == InstructionStep::STEP0 {
            // Sign extenstion of the 24 bits immediate. Offset is this value * 4
            nn |= if nn.is_bit_set(23) { 0xFF000000 } else { 0 };
            let offset: i32 = (nn as i32) << 2;

            self.arm_instruction_queue.clear();
            req.bus_cycle = BusCycle::NONSEQUENTIAL;
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP1;

            // If the operation is branch and link, store the next instruction to be used in the
            // link register
            if opcode == 1 {
                self.rf.write_register(14, current_pc);
            }

            // Increment only by 4 due to the automatic increase of the pc at the end of the
            // instruction
            self.rf
                .write_register(15, (current_pc as i32 + offset + 8) as u32);

        // Refill the pipeline in the next two steps
        } else if self.instruction_step == InstructionStep::STEP1 {
            req.address = current_pc;
            self.instruction_step = InstructionStep::STEP2;
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.address = current_pc.wrapping_add(4);
            self.rf
                .write_register(15, self.rf.get_register(15, 0).wrapping_sub(4));
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for instructin type ARM_BRANCH_AND_EXCHANGE");
        }
    }

    /// arm7_tdmi::arm_single_data_transfer
    ///
    /// Function to handle all the single data transfer instructions (LDR, STR, LDRB, STRB)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn arm_single_data_transfer(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        // condition to be checked for the instruction to be executed
        let condition = self.arm_current_execute.get_range(31, 28);

        // load flag, 1 for a load operation
        let l_flag = self.arm_current_execute.get_range(20, 20);

        // operand register
        let rn = self.arm_current_execute.get_range(19, 16);

        // destination register
        let rd = self.arm_current_execute.get_range(15, 12);

        // p flag, 1 for pre-increment
        let p_flag = self.arm_current_execute.get_range(24, 24);

        // when p is 1, w flag: 1 for increment the base register after the transaction
        // when p is 0, t flag: 1 to make the request non privileged on the bus
        let tw_flag = self.arm_current_execute.get_range(21, 21);

        // when b is 1, a byte is requested
        let b_flag = self.arm_current_execute.get_range(22, 22);

        let offset;
        let address_to_mem;
        let mut address_to_write = 0;

        if !self.rf.check_condition_code(condition) {
            return;
        }

        // Common between load and store: during step1, the bus transaction to store/load is
        // generated, so in this step we need the address to be used.
        if self.instruction_step == InstructionStep::STEP1 {
            let i_flag = self.arm_current_execute.get_range(25, 25);
            let u_flag = self.arm_current_execute.get_range(23, 23);
            let immediate = self.arm_current_execute.get_range(11, 0);
            let shift_amount = self.arm_current_execute.get_range(11, 7);
            let shift_type = self.arm_current_execute.get_range(6, 5);
            let rm = self.arm_current_execute.get_range(3, 0);

            address_to_mem = self.rf.get_register(rn, 8);

            if i_flag == 0 {
                offset = immediate;
            } else {
                if rm == 15 {
                    panic!("Cannot use r15 as shift register in ARM_SINGLE_DATA_TRANSFER");
                }
                (offset, _, _) = barrel_shifter(
                    self.rf.get_register(rm, 0),
                    shift_type,
                    shift_amount,
                    self.rf.is_flag_set(&ConditionCodeFlag::C),
                );
            }

            if u_flag == 1 {
                address_to_write = address_to_mem.wrapping_add(offset);
            } else {
                address_to_write = address_to_mem.wrapping_sub(offset);
            }

            if p_flag == 1 {
                req.address = address_to_write;
            } else if tw_flag == 1 {
                req.address = address_to_mem;
                req.n_trans = BusSignal::LOW;
            }

            if b_flag == 1 {
                req.mas = TransferSize::BYTE;
            }
        }

        // Load instruction
        if l_flag == 1 {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                self.data_is_fetch = false;
                req.bus_cycle = BusCycle::INTERNAL;
                // Post increment of the base register
                if p_flag == 0 || tw_flag == 1 {
                    self.rf.write_register(rn, address_to_write);
                }
                self.instruction_step = InstructionStep::STEP2;
            } else if self.instruction_step == InstructionStep::STEP2 {
                // Write data back to the destination register
                let mut data_to_write = rsp.data;
                let offset = self.last_used_address % 4;

                // If only one byte is requested, the correct byte must be extracted from the
                // received data, taking into account that we are only working in little endian
                // mode
                if b_flag == 1 {
                    data_to_write = data_to_write.get_range(offset * 8 + 7, offset * 8);

                // If the required address was not word aligned, a rotation should be applied
                } else {
                    data_to_write = data_to_write.rotate_right(offset * 8);
                }

                // Update the destination register
                self.rf.write_register(rd, data_to_write);
                self.data_is_fetch = false;

                // If destination is r15, then the pipeline is to be filled again
                if rd == 15 {
                    self.arm_instruction_queue.clear();
                    req.bus_cycle = BusCycle::NONSEQUENTIAL;
                    self.instruction_step = InstructionStep::STEP3;
                } else {
                    req.bus_cycle = BusCycle::SEQUENTIAL;
                    self.instruction_step = InstructionStep::STEP0;
                }
            } else if self.instruction_step == InstructionStep::STEP3 {
                req.address = self.rf.get_register(15, 0);
                self.instruction_step = InstructionStep::STEP4;
            } else if self.instruction_step == InstructionStep::STEP4 {
                req.address = self.rf.get_register(15, 4);
                self.rf
                    .write_register(15, self.rf.get_register(15, 0).wrapping_sub(4));
                self.instruction_step = InstructionStep::STEP0;
            } else if self.instruction_step == InstructionStep::STEP4 {
            } else {
                panic!("Wrong step for instructin type ARM_LOAD");
            }

        // Store instruction
        } else {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                req.data = self.rf.get_register(rd, 12);

                // If only one byte is to be moved, copy the byte over all the 32 lines of the bus
                if b_flag == 1 {
                    let byte = req.data & 0xff;
                    req.data = byte | (byte << 8) | (byte << 16) | (byte << 24);
                }

                if p_flag == 0 || tw_flag == 1 {
                    self.rf.write_register(rn, address_to_write);
                }

                req.nr_w = BusSignal::HIGH;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong step for instructin type ARM_STORE");
            }
        }
    }

    /// arm7_tdmi::arm_hw_transfer
    ///
    /// Function to handle all the halfword data transfer instructions, both with immediate as
    /// operand and with registers
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn arm_hw_transfer(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let condition = self.arm_current_execute.get_range(31, 28);
        let p_flag = self.arm_current_execute.get_range(24, 24);
        let u_flag = self.arm_current_execute.get_range(23, 23);
        let i_flag = self.arm_current_execute.get_range(22, 22);
        let w_flag = self.arm_current_execute.get_range(21, 21);
        let l_flag = self.arm_current_execute.get_range(20, 20);
        let rn = self.arm_current_execute.get_range(19, 16);
        let rd = self.arm_current_execute.get_range(15, 12);
        let imm_offset_h = self.arm_current_execute.get_range(11, 8);
        let opcode = self.arm_current_execute.get_range(6, 5);
        let rm = self.arm_current_execute.get_range(3, 0);
        let mut address_to_write = 0;

        if !self.rf.check_condition_code(condition) {
            return;
        }

        let address_to_mem = self.rf.get_register(rn, 8);
        let offset;

        if opcode == 0 {
            panic!("Opcode reserved for SWP instruction");
        }

        if self.instruction_step == InstructionStep::STEP1 {
            offset = if i_flag == 1 {
                (imm_offset_h << 4) | rm
            } else {
                if rm == 15 {
                    panic!("Cannot use r15 as shift register in ARM_SINGLE_DATA_TRANSFER");
                }
                self.rf.get_register(rm, 0)
            };

            address_to_write = if u_flag == 1 {
                address_to_mem.wrapping_add(offset)
            } else {
                address_to_mem.wrapping_sub(offset)
            };

            req.address = if p_flag == 1 {
                address_to_write
            } else {
                address_to_mem
            }
        }

        if l_flag == 1 {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                self.data_is_fetch = false;
                req.bus_cycle = BusCycle::INTERNAL;

                req.mas = if opcode == 2 {
                    TransferSize::BYTE
                } else {
                    TransferSize::HALFWORD
                };

                // Post increment of the base register
                if p_flag == 0 || w_flag == 1 {
                    self.rf.write_register(rn, address_to_write);
                }

                self.instruction_step = InstructionStep::STEP2;
            } else if self.instruction_step == InstructionStep::STEP2 {
                let mut data_to_write = rsp.data;
                let offset = self.last_used_address % 4;

                if opcode == 2 {
                    data_to_write = data_to_write.get_range(offset * 8 + 7, offset * 8);
                    if data_to_write.is_bit_set(7) {
                        data_to_write |= 0xffffff00;
                    } else {
                        data_to_write &= 0x000000ff;
                    }
                } else {
                    if offset == 2 {
                        data_to_write >>= 16;
                    }
                    if data_to_write.is_bit_set(15) && opcode == 3 {
                        data_to_write |= 0xffff0000;
                    } else {
                        data_to_write &= 0x0000ffff;
                    }
                }

                // Update the destination register
                self.rf.write_register(rd, data_to_write);
                self.data_is_fetch = false;

                // If destination is r15, then the pipeline is to be filled again
                if rd == 15 {
                    self.arm_instruction_queue.clear();
                    req.bus_cycle = BusCycle::NONSEQUENTIAL;
                    self.instruction_step = InstructionStep::STEP3;
                } else {
                    req.bus_cycle = BusCycle::SEQUENTIAL;
                    self.instruction_step = InstructionStep::STEP0;
                }
            } else if self.instruction_step == InstructionStep::STEP3 {
                req.address = self.rf.get_register(15, 0);
                self.instruction_step = InstructionStep::STEP4;
            } else if self.instruction_step == InstructionStep::STEP4 {
                req.address = self.rf.get_register(15, 4);
                self.rf
                    .write_register(15, self.rf.get_register(15, 0).wrapping_sub(4));
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong step for instructin type ARM_LOAD_HW");
            }
        } else {
            // strh
            if opcode == 1 {
                if self.instruction_step == InstructionStep::STEP0 {
                    req.bus_cycle = BusCycle::NONSEQUENTIAL;
                    self.instruction_step = InstructionStep::STEP1;
                } else if self.instruction_step == InstructionStep::STEP1 {
                    // Post increment of the base register
                    if p_flag == 0 || w_flag == 1 {
                        self.rf.write_register(rn, address_to_write);
                    }

                    req.mas = TransferSize::HALFWORD;
                    req.data = self.rf.get_register(rd, 12);
                    req.data = (req.data & 0xffff) | (req.data << 16);
                    req.nr_w = BusSignal::HIGH;
                    self.instruction_step = InstructionStep::STEP0;
                    self.data_is_fetch = false;
                } else {
                    panic!("Wrong step for instructin type ARM_STRH");
                }

            // ldrd
            } else if opcode == 2 {
                if self.instruction_step == InstructionStep::STEP0 {
                    req.bus_cycle = BusCycle::NONSEQUENTIAL;
                    self.instruction_step = InstructionStep::STEP1;
                } else if self.instruction_step == InstructionStep::STEP1 {
                    if req.address % 8 != 0 {
                        panic!("Address must be double-word aligned in ARM_LDRD");
                    }
                    if rd % 2 != 0 || rd == 14 {
                        panic!("rd must be even and less than 12 in ARM_LDRD");
                    }
                    self.data_is_fetch = false;
                    self.instruction_step = InstructionStep::STEP2;
                } else if self.instruction_step == InstructionStep::STEP2 {
                    self.data_is_fetch = false;
                    req.bus_cycle = BusCycle::INTERNAL;
                    self.rf.write_register(rd, rsp.data);
                    req.address = self.last_used_address + 4;
                    self.instruction_step = InstructionStep::STEP3;
                } else if self.instruction_step == InstructionStep::STEP3 {
                    self.data_is_fetch = false;
                    self.rf.write_register(rd + 1, rsp.data);
                    self.instruction_step = InstructionStep::STEP0;
                } else {
                    panic!("Wrong step for instruction type ARM_LDRD")
                }

            // strd
            } else if opcode == 3 {
                if self.instruction_step == InstructionStep::STEP0 {
                    req.bus_cycle = BusCycle::NONSEQUENTIAL;
                    self.instruction_step = InstructionStep::STEP1;
                } else if self.instruction_step == InstructionStep::STEP1 {
                    req.data = self.rf.get_register(rd, 0);
                    req.nr_w = BusSignal::HIGH;
                    self.instruction_step = InstructionStep::STEP2;
                    self.data_is_fetch = false;
                } else if self.instruction_step == InstructionStep::STEP2 {
                    req.address = self.last_used_address + 4;
                    req.data = self.rf.get_register(rd + 1, 0);
                    req.nr_w = BusSignal::HIGH;
                    self.instruction_step = InstructionStep::STEP0;
                    self.data_is_fetch = false;
                } else {
                    panic!("Wrong step for instructin type ARM_STRD");
                }
            }
        }
    }

    /// arm7_tdmi::arm_swi
    ///
    /// Function to handle all the swi instruction
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    pub fn arm_swi(&mut self, req: &mut MemoryRequest) {
        let condition = self.arm_current_execute.get_range(31, 28);

        if !self.rf.check_condition_code(condition) {
            return;
        }

        if self.instruction_step == InstructionStep::STEP0 {
            self.arm_instruction_queue.clear();
            req.bus_cycle = BusCycle::NONSEQUENTIAL;
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP1;
        } else if self.instruction_step == InstructionStep::STEP1 {
            let current_cpsr = self.rf.get_cpsr();
            let _ = self
                .rf
                .write_cpsr((current_cpsr & 0xffffffe0) | (OperatingMode::SUPERVISOR as u32));
            let _ = self.rf.write_spsr(current_cpsr);

            self.rf.write_register(14, self.rf.get_register(15, 4));
            self.rf.write_register(15, 0x04);

            req.address = self.rf.get_register(15, 4);
            self.instruction_step = InstructionStep::STEP2;
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.address = self.rf.get_register(15, 8);
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for instructin type ARM_SWI");
        }
    }

    /// arm7_tdmi::arm_undefined
    ///
    /// Function to handle the undefined instruction, jumping to the proper exception address
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    pub fn arm_undefined(&mut self, req: &mut MemoryRequest) {
        let condition = self.arm_current_execute.get_range(31, 28);

        if !self.rf.check_condition_code(condition) {
            return;
        }

        if self.instruction_step == InstructionStep::STEP0 {
            self.arm_instruction_queue.clear();
            req.bus_cycle = BusCycle::NONSEQUENTIAL;
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP1;
        } else if self.instruction_step == InstructionStep::STEP1 {
            let current_cpsr = self.rf.get_cpsr();
            let _ = self
                .rf
                .write_cpsr((current_cpsr & 0xffffffe0) | (OperatingMode::UND as u32));
            let _ = self.rf.write_spsr(current_cpsr);

            self.rf.write_register(14, self.rf.get_register(15, 4));
            self.rf.write_register(15, 0);

            req.address = self.rf.get_register(15, 4);
            self.instruction_step = InstructionStep::STEP2;
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.address = self.rf.get_register(15, 8);
            req.bus_cycle = BusCycle::INTERNAL;
            self.instruction_step = InstructionStep::STEP3;
        } else if self.instruction_step == InstructionStep::STEP3 {
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for instructin type ARM_UND");
        }
    }

    /// arm7_tdmi::arm_psr_transfer_mrs
    ///
    /// Function to handle the mrs instruction. This transfer PSR content to a register
    pub fn arm_psr_transfer_mrs(&mut self) {
        let condition = self.arm_current_execute.get_range(31, 28);
        let psr_flag = self.arm_current_execute.get_range(22, 22);
        let rd = self.arm_current_execute.get_range(15, 12);

        if !self.rf.check_condition_code(condition) {
            return;
        }

        if rd == 15 {
            panic!("Cannot specify r15 as destination address for mrs")
        }

        let source_register = if psr_flag == 0 {
            self.rf.get_cpsr()
        } else {
            self.rf.get_spsr()
        };

        self.rf.write_register(rd, source_register);
    }

    /// arm7_tdmi::arm_psr_transfer_msr
    ///
    /// Function to handle the msr instruction. It applies some appropriate modifications either to
    /// cpsr or spsr.
    pub fn arm_psr_transfer_msr(&mut self) {
        let condition = self.arm_current_execute.get_range(31, 28);
        let i_flag = self.arm_current_execute.get_range(25, 25);
        let psr_flag = self.arm_current_execute.get_range(22, 22);
        let f_flag = self.arm_current_execute.get_range(19, 19);
        let c_flag = self.arm_current_execute.get_range(16, 16);
        let rm = self.arm_current_execute.get_range(3, 0);
        let shift = self.arm_current_execute.get_range(11, 8);
        let imm = self.arm_current_execute.get_range(7, 0);

        if !self.rf.check_condition_code(condition) {
            return;
        }

        let source_operand = if i_flag == 0 {
            if rm == 15 {
                panic!("Cannot specify r15 as source register for msr")
            }
            self.rf.get_register(rm, 0)
        } else {
            imm.rotate_right(shift * 2)
        };

        let mut to_modify = if psr_flag == 1 {
            self.rf.get_spsr()
        } else {
            self.rf.get_cpsr()
        };

        if c_flag == 1 && self.rf.get_mode() != OperatingMode::USER {
            to_modify = to_modify & 0xffffff00;
            to_modify = to_modify | (source_operand & 0x000000ff);
        }

        if f_flag == 1 {
            to_modify = to_modify & 0x0fffffff;
            to_modify = to_modify | (source_operand & 0xf0000000);
        }

        if psr_flag == 1 {
            let _ = self.rf.write_spsr(to_modify);
        } else {
            let _ = self.rf.write_cpsr(to_modify);
        }
    }

    /// arm7_tdmi::arm_single_data_swap
    ///
    /// Function to implement the swp instruction
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn arm_single_data_swap(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let condition = self.arm_current_execute.get_range(31, 28);
        let b_flag = self.arm_current_execute.get_range(22, 22);
        let rn = self.arm_current_execute.get_range(19, 16);
        let rd = self.arm_current_execute.get_range(15, 12);
        let rm = self.arm_current_execute.get_range(3, 0);
        let offset = self.last_used_address % 4;

        if rd == 15 || rm == 15 || rn == 15 {
            panic!("Cannot use r15 in SWP instruction");
        }

        if !self.rf.check_condition_code(condition) {
            return;
        }

        if self.instruction_step == InstructionStep::STEP0 {
            req.bus_cycle = BusCycle::NONSEQUENTIAL;
            self.instruction_step = InstructionStep::STEP1;
        } else if self.instruction_step == InstructionStep::STEP1 {
            self.data_is_fetch = false;

            req.address = self.rf.get_register(rn, 0);
            req.mas = if b_flag == 0 {
                TransferSize::WORD
            } else {
                TransferSize::BYTE
            };
            req.n_opc = BusSignal::HIGH;
            req.lock = BusSignal::HIGH;
            req.bus_cycle = BusCycle::NONSEQUENTIAL;

            self.last_used_address = req.address;
            self.instruction_step = InstructionStep::STEP2;
        } else if self.instruction_step == InstructionStep::STEP2 {
            self.data_is_fetch = false;

            req.address = self.rf.get_register(rn, 0);
            if b_flag == 0 {
                self.rf.write_register(rd, rsp.data);
                req.mas = TransferSize::WORD;
                req.data = self.rf.get_register(rm, 0);
            } else {
                let mut data_to_write = rsp.data;
                data_to_write = data_to_write.get_range(offset * 8 + 7, offset * 8);
                self.rf.write_register(rd, data_to_write);
                req.mas = TransferSize::BYTE;
                req.data = self.rf.get_register(rm, 0).get_range(7, 0);
                req.data = req.data | (req.data << 8) | (req.data << 16) | (req.data << 24);
            };
            req.nr_w = BusSignal::HIGH;
            req.n_opc = BusSignal::HIGH;
            req.lock = BusSignal::HIGH;
            req.bus_cycle = BusCycle::INTERNAL;

            self.instruction_step = InstructionStep::STEP3;
        } else if self.instruction_step == InstructionStep::STEP3 {
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong instruction step for SWP");
        }
    }

    /// arm7_tdmi::arm_multiply
    ///
    /// Function to handle the mul and mla instructions
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    pub fn arm_multiply(&mut self, req: &mut MemoryRequest) {
        let condition = self.arm_current_execute.get_range(31, 28);
        let opcode = self.arm_current_execute.get_range(24, 21);
        let s_flag = self.arm_current_execute.get_range(20, 20);
        let rd = self.arm_current_execute.get_range(19, 16);
        let rn = self.arm_current_execute.get_range(15, 12);
        let rs = self.arm_current_execute.get_range(11, 8);
        let rm = self.arm_current_execute.get_range(3, 0);

        if !self.rf.check_condition_code(condition) {
            return;
        }

        req.bus_cycle = BusCycle::INTERNAL;

        if self.instruction_step == InstructionStep::STEP0 {
            let op_s = self.rf.get_register(rs, 0);

            self.instruction_step = InstructionStep::STEP1;

            let leading_zeros = op_s.leading_zeros();

            self.instruction_counter_step = if leading_zeros >= 24 {
                1
            } else if leading_zeros >= 16 {
                2
            } else if leading_zeros >= 8 {
                3
            } else {
                4
            };

            self.instruction_counter_step += if opcode == 0 {
                0
            } else if opcode == 5 || opcode == 7 {
                2
            } else {
                1
            };
        } else if self.instruction_step == InstructionStep::STEP1 {
            self.instruction_counter_step -= 1;

            if self.instruction_counter_step == 0 {
                let op_s = self.rf.get_register(rs, 0);
                let op_m = self.rf.get_register(rm, 0);
                let op_n = self.rf.get_register(rn, 0);
                let op_d = self.rf.get_register(rd, 0);

                if opcode <= 1 {
                    let result = if opcode == 0 {
                        ((op_s as u64) * (op_m as u64)) as u32
                    } else {
                        ((op_s as u64) * (op_m as u64) + (op_n as u64)) as u32
                    };

                    self.rf.write_register(rd, result);

                    if s_flag == 1 {
                        self.rf.write_z(result == 0);
                        self.rf.write_n(result.is_bit_set(31));
                    }
                } else {
                    let result = if opcode == 4 {
                        (op_s as u64) * (op_m as u64)
                    } else if opcode == 5 {
                        (op_s as u64) * (op_m as u64) + (op_n as u64 | ((op_d as u64) << 32))
                    } else if opcode == 6 {
                        ((op_s as i32 as i64) * (op_m as i32 as i64)) as u64
                    } else if opcode == 7 {
                        ((op_s as i32 as i64) * (op_m as i32 as i64)
                            + ((op_n as u64 | (op_d as u64) << 32) as i32 as i64))
                            as u64
                    } else {
                        panic!("Opcode cannot be used in MUL")
                    };

                    self.rf.write_register(rd, result.get_range(63, 32) as u32);
                    self.rf.write_register(rn, result.get_range(31, 0) as u32);

                    if s_flag == 1 {
                        self.rf.write_z(result == 0);
                        self.rf.write_n(result.is_bit_set(63));
                    }
                }

                self.instruction_step = InstructionStep::STEP0;
                req.bus_cycle = BusCycle::SEQUENTIAL;
            }
        } else {
            panic!("Wrong step for instructin type ARM_MUL");
        }
    }
}
