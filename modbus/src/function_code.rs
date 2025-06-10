#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FunctionCode {
    ReadCoils = 1,
    ReadDiscreteInputs = 2,
    ReadHoldingRegisters = 3,
    ReadInputRegisters = 4,
    WriteSingleCoil = 5,
    WriteSingleHoldingRegister = 6,
    WriteMultipleCoils = 15,
    WriteMultipleHoldingRegisters = 16,
    MaskWriteHoldingRegister = 22,
    ModbusEncapsulatedInterface = 43,
    Error(u8),
    Unknown(u8),
}

impl FunctionCode {
    pub fn as_err(self) -> Self {
        FunctionCode::Error(u8::from(self) | 128u8)
    }
}

impl From<u8> for FunctionCode {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::ReadCoils,
            2 => Self::ReadDiscreteInputs,
            3 => Self::ReadHoldingRegisters,
            4 => Self::ReadInputRegisters,
            5 => Self::WriteSingleCoil,
            6 => Self::WriteSingleHoldingRegister,
            15 => Self::WriteMultipleCoils,
            16 => Self::WriteMultipleHoldingRegisters,
            22 => Self::MaskWriteHoldingRegister,
            43 => Self::ModbusEncapsulatedInterface,
            _ => {
                if value & 128 != 0 {
                    Self::Error(value)
                } else {
                    Self::Unknown(value)
                }
            }
        }
    }
}

impl From<FunctionCode> for u8 {
    fn from(value: FunctionCode) -> Self {
        match value {
            FunctionCode::ReadCoils => 1,
            FunctionCode::ReadDiscreteInputs => 2,
            FunctionCode::ReadHoldingRegisters => 3,
            FunctionCode::ReadInputRegisters => 4,
            FunctionCode::WriteSingleCoil => 5,
            FunctionCode::WriteSingleHoldingRegister => 6,
            FunctionCode::WriteMultipleCoils => 15,
            FunctionCode::WriteMultipleHoldingRegisters => 16,
            FunctionCode::MaskWriteHoldingRegister => 22,
            FunctionCode::ModbusEncapsulatedInterface => 43,
            FunctionCode::Error(value) => value,
            FunctionCode::Unknown(value) => value,
        }
    }
}

impl PartialEq for FunctionCode {
    fn eq(&self, other: &Self) -> bool {
        u8::from(*self) == u8::from(*other)
    }
}
