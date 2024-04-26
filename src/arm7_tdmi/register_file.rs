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

#[derive(Debug, PartialEq, Eq)]
pub struct RegisterFile {
    registers: Vec<u32>,
    fiq_bank: Vec<u32>,
    svc_bank: Vec<u32>,
    abt_bank: Vec<u32>,
    irq_bank: Vec<u32>,
    und_bank: Vec<u32>,
    cpsr: u32,
    spsr: Vec<u32>,
    current_mode: OperatingMode,
}

impl RegisterFile {
    /// RegisterFile::new
    ///
    /// Create the empty register file, with all the registers set to 0.
    /// This behaviour might not be the definitive one.
    fn new() -> Self {
        Self {
            registers: vec![0; 16],
            fiq_bank: vec![0; 7],
            svc_bank: vec![0; 2],
            abt_bank: vec![0; 2],
            irq_bank: vec![0; 2],
            und_bank: vec![0; 2],
            cpsr: 0,
            spsr: vec![0; 5],
            current_mode: OperatingMode::SYSTEM,
        }
    }

    /// RegisterFile::get_register
    ///
    /// Get one of the 16 general purpose registers, using the correct
    /// bank depending on the working mode.
    ///
    /// @param index [u32]: which of the registers to use
    /// @return [u32]: register
    fn get_register(&self, index: u32) -> u32 {
        let index = index as usize;
        let mode = self.cpsr.get_range(4, 0);
        match index {
            0..=7 => self.registers[index],
            8..=12 => {
                if self.current_mode == OperatingMode::FIQ {
                    self.fiq_bank[index - 8]
                } else {
                    self.registers[index]
}
            }
            13..=14 => match mode {
                mode if mode == OperatingMode::SYSTEM.value()
                    || mode == OperatingMode::USER.value() =>
                {
                    self.registers[index]
                }
                mode if mode == OperatingMode::FIQ.value() => self.fiq_bank[index - 8],
                mode if mode == OperatingMode::SUPERVISOR.value() => self.svc_bank[index - 13],
                mode if mode == OperatingMode::ABORT.value() => self.abt_bank[index - 13],
                mode if mode == OperatingMode::IRQ.value() => self.irq_bank[index - 13],
                mode if mode == OperatingMode::UND.value() => self.und_bank[index - 13],
                _ => panic!("Illegal mode {:#05b} in cpsr", mode),
            },
            15 => self.registers[15],
            _ => panic!("Wrong index used in `get_register`: {}", index),
        }
    }

    /// RegisterFile::write_register
    ///
    /// Write one of the 16 general purpose registers, using the correct
    /// bank depending on the working mode.
    ///
    /// @param index [u32]: which of the registers to use
    /// @param value [u32]: new content of the register
    fn write_register(&mut self, index: u32, value: u32) {
        let mode = self.cpsr.get_range(4, 0);
        let index = index as usize;
        match index {
            0..=7 => self.registers[index] = value,
            8..=12 => {
                if self.current_mode == OperatingMode::FIQ {
                    self.fiq_bank[index - 8] = value
                } else {
                    self.registers[index] = value
                }
            }
            13..=14 => match mode {
                mode if mode == OperatingMode::FIQ.value() => self.fiq_bank[index - 8] = value,
                mode if mode == OperatingMode::SUPERVISOR.value() => {
                    self.svc_bank[index - 13] = value
                }
                mode if mode == OperatingMode::ABORT.value() => self.abt_bank[index - 13] = value,
                mode if mode == OperatingMode::IRQ.value() => self.irq_bank[index - 13] = value,
                mode if mode == OperatingMode::UND.value() => self.und_bank[index - 13] = value,
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
    fn get_cpsr(&self) -> u32 {
        self.cpsr
    }

    /// RegisterFile::write_cpsr
    ///
    /// Modify the content of cpsr
    ///
    /// @param value [u32]: value to use
    /// @return [Result<(), ()>]: Err if the operating mode is not correct, Ok otherwise
    fn write_cpsr(&mut self, value: u32) -> Result<(), ()> {
        let mode = value.get_range(4, 0);
        if mode != OperatingMode::SYSTEM.value()
            && mode != OperatingMode::USER.value()
            && mode != OperatingMode::FIQ.value()
            && mode != OperatingMode::IRQ.value()
            && mode != OperatingMode::SUPERVISOR.value()
            && mode != OperatingMode::ABORT.value()
            && mode != OperatingMode::UND.value()
        {
            return Err(());
        }
        self.cpsr = value;
        Ok(())
    }

    /// RegisterFile::get_spsr
    ///
    /// Read spsr register, using the correct value depending on the
    /// working mode
    ///
    /// @return [u32]: spsr
    fn get_spsr(&mut self) -> u32 {
        let mode = self.cpsr.get_range(4, 0);
        match mode {
            mode if mode == OperatingMode::FIQ.value() => self.spsr[0],
            mode if mode == OperatingMode::SUPERVISOR.value() => self.spsr[1],
            mode if mode == OperatingMode::ABORT.value() => self.spsr[2],
            mode if mode == OperatingMode::IRQ.value() => self.spsr[3],
            mode if mode == OperatingMode::UND.value() => self.spsr[4],
            _ => 0,
        }
    }

    /// RegisterFile::write_spsr
    ///
    /// Modify the value of spsr register, using the correct bank depending on the
    /// working mode
    ///
    /// @param value [u32]: value to use
    /// @return [Result<(), ()>]: Err if the operating mode is not correct, Ok otherwise
    fn write_spsr(&mut self, value: u32) -> Result<(), ()> {
        let mode = value.get_range(4, 0);
        if mode != OperatingMode::SYSTEM.value()
            && mode != OperatingMode::USER.value()
            && mode != OperatingMode::FIQ.value()
            && mode != OperatingMode::IRQ.value()
            && mode != OperatingMode::SUPERVISOR.value()
            && mode != OperatingMode::ABORT.value()
            && mode != OperatingMode::UND.value()
        {
            return Err(());
        }

        let mode = self.cpsr.get_range(4, 0);
        match mode {
            mode if mode == OperatingMode::FIQ.value() => self.spsr[0] = value,
            mode if mode == OperatingMode::SUPERVISOR.value() => self.spsr[1] = value,
            mode if mode == OperatingMode::ABORT.value() => self.spsr[2] = value,
            mode if mode == OperatingMode::IRQ.value() => self.spsr[3] = value,
            mode if mode == OperatingMode::UND.value() => self.spsr[4] = value,
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod test_register_file {

    use crate::arm7_tdmi::register_file::RegisterFile;
    use crate::arm7_tdmi::OperatingMode;

    #[test]
    fn test_registers() {
        let mut rf = RegisterFile::new();

        assert_eq!(rf.write_cpsr(OperatingMode::USER.value()), Ok(()));
        rf.write_register(0, 0x0a);
        assert_eq!(0x0a, rf.get_register(0));

        rf.write_register(14, 0x7ac0);
        assert_eq!(0x7ac0, rf.get_register(14));

        rf.write_register(15, 0x1001);
        assert_eq!(0x1001, rf.get_register(15));

        assert_eq!(rf.write_cpsr(OperatingMode::IRQ.value()), Ok(()));
        assert_eq!(rf.write_spsr(OperatingMode::SYSTEM.value()), Ok(()));
        assert_eq!(0x0a, rf.get_register(0));
        assert_eq!(0x0, rf.get_register(14));
        assert_eq!(OperatingMode::SYSTEM.value(), rf.get_spsr());

        rf.write_register(14, 0xbe11);
        assert_eq!(0xbe11, rf.get_register(14));
        assert_eq!(0x1001, rf.get_register(15));

        assert_eq!(rf.write_cpsr(OperatingMode::SYSTEM.value()), Ok(()));
        assert_eq!(0x0a, rf.get_register(0));
        assert_eq!(0x7ac0, rf.get_register(14));
        assert_eq!(0x1001, rf.get_register(15));

        assert_eq!(rf.write_cpsr(OperatingMode::SUPERVISOR.value()), Ok(()));
        assert_eq!(0x0a, rf.get_register(0));
        assert_eq!(0, rf.get_register(14));

        // Makes the test panic due to non valid mode.
        assert_eq!(rf.write_cpsr(0), Err(()));
    }
}
