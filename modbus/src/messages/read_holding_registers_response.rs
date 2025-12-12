use std::borrow::Cow;

use crate::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadHoldingRegistersResponse<'a> {
    pub values: Cow<'a, [u16]>,
}

impl<'a> Encodable for ReadHoldingRegistersResponse<'a> {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8((self.values.len() * 2).try_into()?);
        encoder.write_registers(&self.values);
        Ok(())
    }
}

impl<'a> Decodable<Self> for ReadHoldingRegistersResponse<'a> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        let byte_length = decoder.read_u8()?;
        if byte_length % 2 != 0 {
            return Err(DecodeError::InvalidData("Byte length in not a multiple of 2".into()));
        }
        Ok(Self {
            values: decoder.read_registers((byte_length / 2) as usize)?.into(),
        })
    }
}
