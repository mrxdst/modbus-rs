use std::borrow::Cow;

use crate::encoding::*;

#[derive(PartialEq, Debug)]
pub struct WriteMultipleHoldingRegistersRequest<'a> {
    pub address: u16,
    pub values: Cow<'a, [u16]>,
}

impl<'a> Encodable for WriteMultipleHoldingRegistersRequest<'a> {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u16(self.address);
        encoder.write_u16(self.values.len().try_into()?);
        encoder.write_u8((self.values.len() * 2).try_into()?);
        encoder.write_registers(&self.values);
        Ok(())
    }
}

impl<'a> Decodable<Self> for WriteMultipleHoldingRegistersRequest<'a> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        let address = decoder.read_u16()?;
        let length = decoder.read_u16()?;
        let byte_length = decoder.read_u8()?;
        if length as usize * 2 != byte_length as usize {
            return Err(DecodeError::InvalidData("Byte length mismatch".into()));
        }
        Ok(Self {
            address,
            values: decoder.read_registers(length as usize)?.into(),
        })
    }
}
