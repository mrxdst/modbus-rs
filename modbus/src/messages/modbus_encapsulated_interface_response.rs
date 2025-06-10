use std::borrow::Cow;

use crate::{encoding::*, modbus_encapsulated_interface::ModbusEncapsulatedInterfaceType};

#[derive(PartialEq, Debug)]
pub struct ModbusEncapsulatedInterfaceResponse<'a> {
    pub kind: ModbusEncapsulatedInterfaceType,
    pub data: Cow<'a, [u8]>,
}

impl<'a> Encodable for ModbusEncapsulatedInterfaceResponse<'a> {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8(self.kind.into());
        encoder.write_bytes(&self.data);
        return Ok(());
    }
}

impl<'a> Decodable<Self> for ModbusEncapsulatedInterfaceResponse<'a> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        let kind = decoder.read_u8()?.into();
        let data = decoder.read_bytes(decoder.remaining())?.into();

        return Ok(Self { kind, data });
    }
}
