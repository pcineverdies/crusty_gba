use crate::arm7_tdmi::OperatingMode;
use crate::common::BitOperation;

/// RegisterFile struct
///
/// Defines the global registers which can be used by the cpu
/// Registers are banked in different ways depending on the
/// operating mode in which the CPU is working. The operating
/// mode is defined by the 5 MSBs of cpsr.
///
/// TODO:   investigate if some changes are to be done to the
///         register file in thumb mode.
///
/// From gbatek/arm-cpu-register-set:
///
/// System/User FIQ       Supervisor Abort     IRQ       Undefined
/// --------------------------------------------------------------
/// R0          R0        R0         R0        R0        R0
/// R1          R1        R1         R1        R1        R1
/// R2          R2        R2         R2        R2        R2
/// R3          R3        R3         R3        R3        R3
/// R4          R4        R4         R4        R4        R4
/// R5          R5        R5         R5        R5        R5
/// R6          R6        R6         R6        R6        R6
/// R7          R7        R7         R7        R7        R7
/// --------------------------------------------------------------
/// R8          R8_fiq    R8         R8        R8        R8
/// R9          R9_fiq    R9         R9        R9        R9
/// R10         R10_fiq   R10        R10       R10       R10
/// R11         R11_fiq   R11        R11       R11       R11
/// R12         R12_fiq   R12        R12       R12       R12
/// R13 (SP)    R13_fiq   R13_svc    R13_abt   R13_irq   R13_und
/// R14 (LR)    R14_fiq   R14_svc    R14_abt   R14_irq   R14_und
/// R15 (PC)    R15       R15        R15       R15       R15
/// --------------------------------------------------------------
/// CPSR        CPSR      CPSR       CPSR      CPSR      CPSR
/// --          SPSR_fiq  SPSR_svc   SPSR_abt  SPSR_irq  SPSR_und
/// --------------------------------------------------------------

/// register_file::ConditionCodeFlag
///
/// enum to represent the 4 available flags in cpsr [manual, 2.13]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConditionCodeFlag {
    N,
    Z,
    C,
    V,
}

/// register_file::RegisterFile
///
/// structure to define the register file and all its parameters
#[derive(Debug, PartialEq, Eq)]
pub struct RegisterFile {
    registers: Vec<u32>, // registers to use in user mode
    fiq_bank: Vec<u32>,  // register  bank for fiq mode
    svc_bank: Vec<u32>,  // register  bank for svc mode
    abt_bank: Vec<u32>,  // register  bank for abt mode
    irq_bank: Vec<u32>,  // register  bank for irq mode
    und_bank: Vec<u32>,  // register  bank for und mode
    cpsr: u32,           // current cpsr
    spsr: Vec<u32>,      // spsr bank
}

impl RegisterFile {
    /// RegisterFile::new
    ///
    /// Create the empty register file, with all the registers set to 0.
    /// This behaviour might not be the definitive one.
    pub fn new() -> Self {
        let mut registers = vec![0; 16];

        // r15 gets this value so that the first instruction to be fetched is at
        // the expected address
        registers[15] = 0x07fffff8;
        Self {
            registers,
            fiq_bank: vec![0; 7],
            svc_bank: vec![0; 2],
            abt_bank: vec![0; 2],
            irq_bank: vec![0; 2],
            und_bank: vec![0; 2],
            cpsr: 0x00000010,
            spsr: vec![0; 5],
        }
    }

    /// RegisterFile::get_register
    ///
    /// Get one of the 16 general purpose registers, using the correct bank depending on the
    /// current working mode.
    ///
    /// @param index [u32]: which of the registers to use
    /// @param pc_increment [u32]: how much to increment the program counter if it is required
    /// @return [u32]: register
    pub fn get_register(&self, index: u32, pc_increment: u32) -> u32 {
        let index = index as usize;
        let mode = self.cpsr.get_range(4, 0);
        match index {
            0..=7 => self.registers[index],
            8..=12 => match mode {
                mode if mode == OperatingMode::FIQ as u32 => self.fiq_bank[index - 8],
                _ => self.registers[index],
            },
            13..=14 => match mode {
                mode if mode == OperatingMode::SYSTEM as u32 => self.registers[index],
                mode if mode == OperatingMode::USER as u32 => self.registers[index],
                mode if mode == OperatingMode::FIQ as u32 => self.fiq_bank[index - 8],
                mode if mode == OperatingMode::SUPERVISOR as u32 => self.svc_bank[index - 13],
                mode if mode == OperatingMode::ABORT as u32 => self.abt_bank[index - 13],
                mode if mode == OperatingMode::IRQ as u32 => self.irq_bank[index - 13],
                mode if mode == OperatingMode::UND as u32 => self.und_bank[index - 13],
                _ => panic!("Illegal mode {:#05b} in cpsr", mode),
            },
            15 => self.registers[15].wrapping_add(pc_increment),
            _ => panic!("Wrong index used in `get_register`: {}", index),
        }
    }

    /// RegisterFile::write_register
    ///
    /// Write one of the 16 general purpose registers, using the correct bank depending on the
    /// current working mode.
    ///
    /// @param index [u32]: which of the registers to use
    /// @param value [u32]: new content of the register
    pub fn write_register(&mut self, index: u32, value: u32) {
        let mode = self.cpsr.get_range(4, 0);
        let index = index as usize;
        match index {
            0..=7 => self.registers[index] = value,
            8..=12 => match mode {
                mode if mode == OperatingMode::FIQ as u32 => self.fiq_bank[index - 8] = value,
                _ => self.registers[index] = value,
            },
            13..=14 => match mode {
                mode if mode == OperatingMode::FIQ as u32 => self.fiq_bank[index - 8] = value,
                mode if mode == OperatingMode::SUPERVISOR as u32 => {
                    self.svc_bank[index - 13] = value
                }
                mode if mode == OperatingMode::ABORT as u32 => self.abt_bank[index - 13] = value,
                mode if mode == OperatingMode::IRQ as u32 => self.irq_bank[index - 13] = value,
                mode if mode == OperatingMode::UND as u32 => self.und_bank[index - 13] = value,
                _ => self.registers[index] = value,
            },
            15 => self.registers[15] = value,
            _ => panic!("Wrong index used in `get_register`: {}", index),
        };
    }

    /// RegisterFile::get_cpsr
    ///
    /// Read cpsr register
    ///
    /// @return [u32]: cpsr
    pub fn get_cpsr(&self) -> u32 {
        self.cpsr
    }

    /// RegisterFile::write_cpsr
    ///
    /// Modify the content of cpsr
    ///
    /// @param value [u32]: value to use
    /// @return [Result<(), ()>]: Err if the operating mode is not correct, Ok otherwise
    pub fn write_cpsr(&mut self, value: u32) -> Result<(), ()> {
        if !self.is_mode_correct(value) {
            return Err(());
        }
        self.cpsr = value;
        Ok(())
    }

    /// RegisterFile::write_v
    ///
    /// Modify the the v flag
    ///
    /// @param v [bool]: true if v is to be set, false otherwise
    pub fn write_v(&mut self, v: bool) {
        if v {
            self.cpsr = self.cpsr.set_bit(28);
        } else {
            self.cpsr = self.cpsr.clear_bit(28);
        }
    }

    /// RegisterFile::write_n
    ///
    /// Modify the the n flag
    ///
    /// @param n [bool]: true if n is to be set, false otherwise
    pub fn write_n(&mut self, n: bool) {
        if n {
            self.cpsr = self.cpsr.set_bit(31);
        } else {
            self.cpsr = self.cpsr.clear_bit(31);
        }
    }

    /// RegisterFile::write_c
    ///
    /// Modify the the c flag
    ///
    /// @param c [bool]: true if c is to be set, false otherwise
    pub fn write_c(&mut self, c: bool) {
        if c {
            self.cpsr = self.cpsr.set_bit(29);
        } else {
            self.cpsr = self.cpsr.clear_bit(29);
        }
    }

    /// RegisterFile::write_z
    ///
    /// Modify the the Z flag
    ///
    /// @param z [bool]: true if z is to be set, false otherwise
    pub fn write_z(&mut self, z: bool) {
        if z {
            self.cpsr = self.cpsr.set_bit(30);
        } else {
            self.cpsr = self.cpsr.clear_bit(30);
        }
    }

    /// RegisterFile::get_spsr
    ///
    /// Read spsr register, using the correct value depending on the
    /// working mode
    ///
    /// @return [u32]: spsr
    pub fn get_spsr(&mut self) -> u32 {
        let mode = self.cpsr.get_range(4, 0);
        match mode {
            mode if mode == OperatingMode::FIQ as u32 => self.spsr[0],
            mode if mode == OperatingMode::SUPERVISOR as u32 => self.spsr[1],
            mode if mode == OperatingMode::ABORT as u32 => self.spsr[2],
            mode if mode == OperatingMode::IRQ as u32 => self.spsr[3],
            mode if mode == OperatingMode::UND as u32 => self.spsr[4],
            _ => panic!("Cannot read spsr in current mode"),
        }
    }

    /// Registen rFile::is_mode_correct
    ///
    /// Check if the received value of CPSR correpsonds to a correct opearting mode
    ///
    /// @param [u32]: value to check
    /// @return [bool]: result of the check
    fn is_mode_correct(&self, value: u32) -> bool {
        let value = value.get_range(4, 0);
        if value != OperatingMode::SYSTEM as u32
            && value != OperatingMode::USER as u32
            && value != OperatingMode::FIQ as u32
            && value != OperatingMode::IRQ as u32
            && value != OperatingMode::SUPERVISOR as u32
            && value != OperatingMode::ABORT as u32
            && value != OperatingMode::UND as u32
        {
            return false;
        }
        true
    }

    /// RegisterFile::write_spsr
    ///
    /// Modify the value of spsr register, using the correct bank depending on the working mode.
    ///
    /// @param value [u32]: value to use
    /// @return [Result<(), ()>]: Err if the operating mode is not correct, Ok otherwise
    pub fn write_spsr(&mut self, value: u32) -> Result<(), ()> {
        if !self.is_mode_correct(value) {
            return Err(());
        }

        let mode = self.cpsr.get_range(4, 0);
        match mode {
            mode if mode == OperatingMode::FIQ as u32 => self.spsr[0] = value,
            mode if mode == OperatingMode::SUPERVISOR as u32 => self.spsr[1] = value,
            mode if mode == OperatingMode::ABORT as u32 => self.spsr[2] = value,
            mode if mode == OperatingMode::IRQ as u32 => self.spsr[3] = value,
            mode if mode == OperatingMode::UND as u32 => self.spsr[4] = value,
            _ => {}
        }

        Ok(())
    }

    /// RegisterFile::get_flag
    ///
    /// Get the value of the required flag from cpsr
    ///
    /// @param flag [ConditionCodeFlag]: flag to use
    /// @return [bool]: value of the required flag (true if set, false otherwise)
    pub fn is_flag_set(&self, flag: &ConditionCodeFlag) -> bool {
        match flag {
            ConditionCodeFlag::N => self.cpsr.get_range(31, 31) == 1,
            ConditionCodeFlag::Z => self.cpsr.get_range(30, 30) == 1,
            ConditionCodeFlag::C => self.cpsr.get_range(29, 29) == 1,
            ConditionCodeFlag::V => self.cpsr.get_range(28, 28) == 1,
        }
    }

    /// RegisterFile::check_condition_code
    ///
    /// Given the condition code of an instruction, check whether the condition is true or false
    /// using the flags in cspr.
    ///
    /// @param code [u32]: condition code to use, must be in range 0..15
    /// @return [bool]: true if the condition is verified, false otherwise
    pub fn check_condition_code(&self, code: u32) -> bool {
        use ConditionCodeFlag::*;
        match code.get_range(3, 0) {
            0b0000 => self.is_flag_set(&Z),
            0b0001 => !self.is_flag_set(&Z),
            0b0010 => self.is_flag_set(&C),
            0b0011 => !self.is_flag_set(&C),
            0b0100 => self.is_flag_set(&N),
            0b0101 => !self.is_flag_set(&N),
            0b0110 => self.is_flag_set(&V),
            0b0111 => !self.is_flag_set(&V),
            0b1000 => self.is_flag_set(&C) && !self.is_flag_set(&Z),
            0b1001 => !self.is_flag_set(&C) && self.is_flag_set(&Z),
            0b1010 => self.is_flag_set(&N) == self.is_flag_set(&V),
            0b1011 => self.is_flag_set(&N) != self.is_flag_set(&V),
            0b1100 => !self.is_flag_set(&Z) && (self.is_flag_set(&N) == self.is_flag_set(&V)),
            0b1101 => self.is_flag_set(&Z) && (self.is_flag_set(&N) != self.is_flag_set(&V)),
            0b1110 => true,
            0b1111 => true, // Undefined behaviour
            _ => {
                panic!("Provide condition code is not valid")
            }
        }
    }

    /// RegisterFile::get_mode
    ///
    /// Return the curent operating mode, based on the content of cpsr
    ///
    /// @return [OperatingMode]: operating mode
    pub fn get_mode(&mut self) -> OperatingMode {
        let mode = self.cpsr.get_range(4, 0);
        match mode {
            mode if mode == OperatingMode::USER as u32 => OperatingMode::USER,
            mode if mode == OperatingMode::FIQ as u32 => OperatingMode::FIQ,
            mode if mode == OperatingMode::IRQ as u32 => OperatingMode::IRQ,
            mode if mode == OperatingMode::SUPERVISOR as u32 => OperatingMode::SUPERVISOR,
            mode if mode == OperatingMode::ABORT as u32 => OperatingMode::ABORT,
            mode if mode == OperatingMode::UND as u32 => OperatingMode::UND,
            _ => OperatingMode::SYSTEM,
        }
    }
}

#[cfg(test)]
mod test_register_file {

    use crate::arm7_tdmi::register_file::ConditionCodeFlag;
    use crate::arm7_tdmi::register_file::RegisterFile;
    use crate::arm7_tdmi::OperatingMode;

    #[test]
    fn test_registers() {
        let mut rf = RegisterFile::new();

        // Should be able to enter user mode
        assert_eq!(rf.write_cpsr(OperatingMode::USER as u32), Ok(()));

        // r0 should get 0x0a as value
        rf.write_register(0, 0x0a);
        assert_eq!(0x0a, rf.get_register(0, 0));

        // r14 should get 0x7ac0 as value
        rf.write_register(14, 0x7ac0);
        assert_eq!(0x7ac0, rf.get_register(14, 0));

        // r14 should get 0x1001 as value
        rf.write_register(15, 0x1001);
        assert_eq!(0x1001, rf.get_register(15, 0));

        // Should be able to enter IRQ mode
        assert_eq!(rf.write_cpsr(OperatingMode::IRQ as u32), Ok(()));

        // Should be able to write spsr (since now we are in privileged mode)
        assert_eq!(rf.write_spsr(OperatingMode::SYSTEM as u32), Ok(()));

        // r0 is always the same for all the modes
        assert_eq!(0x0a, rf.get_register(0, 0));

        // r14 is not the same as before, so it does not have the same value
        assert_eq!(0x0, rf.get_register(14, 0));

        // Check the previous writing on spsr
        assert_eq!(OperatingMode::SYSTEM as u32, rf.get_spsr());

        // Check the current mode
        assert_eq!(rf.get_mode(), OperatingMode::IRQ);

        // r14 is now modified
        rf.write_register(14, 0xbe11);
        assert_eq!(0xbe11, rf.get_register(14, 0));

        // r15 is alawys the same
        assert_eq!(0x1001, rf.get_register(15, 0));

        // Enter system mode
        assert_eq!(rf.write_cpsr(OperatingMode::SYSTEM as u32), Ok(()));

        // r0 is always the same for all the modes
        assert_eq!(0x0a, rf.get_register(0, 0));

        // SYSTEM and USER share the same registers
        assert_eq!(0x7ac0, rf.get_register(14, 0));
        assert_eq!(0x1001, rf.get_register(15, 0));
        assert_eq!(rf.get_mode(), OperatingMode::SYSTEM);

        // Enter supervisor mode
        assert_eq!(rf.write_cpsr(OperatingMode::SUPERVISOR as u32), Ok(()));
        assert_eq!(rf.get_mode(), OperatingMode::SUPERVISOR);

        // r0 is always the same for all the modes
        assert_eq!(0x0a, rf.get_register(0, 0));

        // r14 is different for each mode
        assert_eq!(0, rf.get_register(14, 0));

        // Cannot write an invalid mode into cpsr
        assert_eq!(rf.write_cpsr(0), Err(()));

        // Enter user mode and set flags N and C
        assert_eq!(
            rf.write_cpsr(0xa0000000 | OperatingMode::USER as u32),
            Ok(())
        );
        assert_eq!(rf.get_mode(), OperatingMode::USER);

        // Only N and C are set
        assert_eq!(rf.is_flag_set(&ConditionCodeFlag::N), true);
        assert_eq!(rf.is_flag_set(&ConditionCodeFlag::Z), false);
        assert_eq!(rf.is_flag_set(&ConditionCodeFlag::C), true);
        assert_eq!(rf.is_flag_set(&ConditionCodeFlag::V), false);
    }

    #[test]
    fn test_condition_code() {
        let mut rf = RegisterFile::new();

        // Z = 1
        let _ = rf.write_cpsr(0x40000010);

        //LS
        assert_eq!(true, rf.check_condition_code(0b1001));
        // EQ
        assert_eq!(true, rf.check_condition_code(0b0000));
        // VS
        assert_eq!(false, rf.check_condition_code(0b0110));
        // always
        assert_eq!(true, rf.check_condition_code(0b1110));
    }
}
