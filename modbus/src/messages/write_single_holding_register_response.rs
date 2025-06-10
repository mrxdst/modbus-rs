use crate::encoding::*;

#[derive(PartialEq, Debug)]
pub struct WriteSingleHoldingRegisterResponse {
    pub address: u16,
    pub value: u16,
}

impl Encodable for WriteSingleHoldingRegisterResponse {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u16(self.address);
        encoder.write_u16(self.value);
        return Ok(());
    }
}

impl Decodable<Self> for WriteSingleHoldingRegisterResponse {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        return Ok(Self {
            address: decoder.read_u16()?,
            value: decoder.read_u16()?,
        });
    }
}
