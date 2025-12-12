use crate::encoding::*;

#[derive(PartialEq, Debug)]
pub struct WriteSingleHoldingRegisterRequest {
    pub address: u16,
    pub value: u16,
}

impl Encodable for WriteSingleHoldingRegisterRequest {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u16(self.address);
        encoder.write_u16(self.value);
        Ok(())
    }
}

impl Decodable<Self> for WriteSingleHoldingRegisterRequest {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self {
            address: decoder.read_u16()?,
            value: decoder.read_u16()?,
        })
    }
}
