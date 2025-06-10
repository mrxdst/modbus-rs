/**
 * Exception codes as defined by the protocol.
 * See the [MODBUS Application Protocol Specification](https://www.modbus.org/docs/Modbus_Application_Protocol_V1_1b3.pdf) for more details.
 */
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ModbusException {
    /// The function code received in the query is not an allowable action for the server.
    IllegalFunction = 1,
    /// The data address received in the query is not an allowable address for the server.
    IllegalDataAddress = 2,
    /// A value contained in the query data field is not an allowable value for server.
    IllegalDataValue = 3,
    /// An unrecoverable error occurred while the server was attempting to perform the requested action.
    ServerDeviceFailure = 4,
    /// The server has accepted the request and is processing it, but a long duration of time will be required to do so.
    Acknowledge = 5,
    /// The server is engaged in processing a long–duration program command.
    ServerDeviceBusy = 6,
    /// The server attempted to read record file, but detected a parity error in the memory.
    MemoryParityError = 8,
    /// Indicates that the gateway was unable to allocate an internal communication path from the input port to the output port for processing the request.
    GatewayPathUnavailable = 10,
    /// Indicates that no response was obtained from the target device.
    GatewayTargetDeviceFailedToRespond = 11,
    /// Something not in the specification.
    Unknown(u8),
}

impl From<u8> for ModbusException {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::IllegalFunction,
            2 => Self::IllegalDataAddress,
            3 => Self::IllegalDataValue,
            4 => Self::ServerDeviceFailure,
            5 => Self::Acknowledge,
            6 => Self::ServerDeviceBusy,
            8 => Self::MemoryParityError,
            10 => Self::GatewayPathUnavailable,
            11 => Self::GatewayTargetDeviceFailedToRespond,
            _ => Self::Unknown(value),
        }
    }
}

impl From<ModbusException> for u8 {
    fn from(value: ModbusException) -> Self {
        match value {
            ModbusException::IllegalFunction => 1,
            ModbusException::IllegalDataAddress => 2,
            ModbusException::IllegalDataValue => 3,
            ModbusException::ServerDeviceFailure => 4,
            ModbusException::Acknowledge => 5,
            ModbusException::ServerDeviceBusy => 6,
            ModbusException::MemoryParityError => 8,
            ModbusException::GatewayPathUnavailable => 10,
            ModbusException::GatewayTargetDeviceFailedToRespond => 11,
            ModbusException::Unknown(value) => value,
        }
    }
}

impl PartialEq for ModbusException {
    fn eq(&self, other: &Self) -> bool {
        u8::from(*self) == u8::from(*other)
    }
}
