use crate::encoding::*;

#[derive(PartialEq, Debug)]
pub struct WriteMultipleHoldingRegistersResponse {
    pub address: u16,
    pub length: u16,
}

impl Encodable for WriteMultipleHoldingRegistersResponse {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u16(self.address);
        encoder.write_u16(self.length);
        return Ok(());
    }
}

impl Decodable<Self> for WriteMultipleHoldingRegistersResponse {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        return Ok(Self {
            address: decoder.read_u16()?,
            length: decoder.read_u16()?,
        });
    }
}
