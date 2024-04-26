mod register_file;

/// arm7_tdmi::OpeartingMode
///
/// enum to represent the different operating modes that the cpu
/// can be into, with respect to [manual, 2.7].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum OperatingMode {
    SYSTEM,
    USER,
    FIQ,
    IRQ,
    SUPERVISOR,
    ABORT,
    UND,
}

impl OperatingMode {
    /// OperatingMode::value
    ///
    /// The 5 msbs of CPSR are used to store the current operating mode.
    /// Each mode has thus a value associated, which can be retrieved
    /// by using this method.
    ///
    /// @return [u32]: value associated to the opearting mode
    fn value(&self) -> u32 {
        match *self {
            OperatingMode::SYSTEM => 0b10000,
            OperatingMode::USER => 0b11111,
            OperatingMode::FIQ => 0b10001,
            OperatingMode::IRQ => 0b10010,
            OperatingMode::SUPERVISOR => 0b10011,
            OperatingMode::ABORT => 0b10111,
            OperatingMode::UND => 0b11011,
        }
    }
}
