use crate::encoding::*;
use crate::modbus_exception::ModbusException;

#[derive(PartialEq, Debug)]
pub struct ExceptionMessage {
    pub code: ModbusException,
}

impl From<ModbusException> for ExceptionMessage {
    fn from(code: ModbusException) -> Self {
        Self { code }
    }
}

impl Encodable for ExceptionMessage {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8(self.code.into());
        return Ok(());
    }
}

impl Decodable<Self> for ExceptionMessage {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        return Ok(Self {
            code: decoder.read_u8()?.into(),
        });
    }
}
