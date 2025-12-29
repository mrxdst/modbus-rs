use std::borrow::Cow;

use crate::encoding::*;

#[derive(PartialEq, Debug)]
pub struct WriteMultipleCoilsRequest<'a> {
    pub address: u16,
    pub values: Cow<'a, [bool]>,
}

impl<'a> Encodable for WriteMultipleCoilsRequest<'a> {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        let length: u16 = self.values.len().try_into()?;
        let byte_length: u8 = self.values.len().div_ceil(8).try_into()?;
        encoder.write_u16(self.address);
        encoder.write_u16(length);
        encoder.write_u8(byte_length);
        encoder.write_bools(&self.values);
        Ok(())
    }
}

impl<'a> Decodable<Self> for WriteMultipleCoilsRequest<'a> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        let address = decoder.read_u16()?;
        let length = decoder.read_u16()?;
        let byte_length = decoder.read_u8()?;

        if (length as u32).div_ceil(8) != byte_length as u32 {
            return Err(DecodeError::InvalidData("Byte length mismatch"));
        }

        Ok(Self {
            address,
            values: decoder.read_bools(length as usize)?.into(),
        })
    }
}
