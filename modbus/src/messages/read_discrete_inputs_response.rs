use std::borrow::Cow;

use crate::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadDiscreteInputsResponse<'a> {
    pub values: Cow<'a, [bool]>,
}

impl<'a> Encodable for ReadDiscreteInputsResponse<'a> {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        let byte_length: u8 = self.values.len().div_ceil(8).try_into()?;
        encoder.write_u8(byte_length);
        encoder.write_bools(&self.values);
        Ok(())
    }
}

impl<'a> Decodable<Self> for ReadDiscreteInputsResponse<'a> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        let byte_length = decoder.read_u8()? as usize;
        Ok(Self {
            values: decoder.read_bools(byte_length * 8)?.into(),
        })
    }
}
