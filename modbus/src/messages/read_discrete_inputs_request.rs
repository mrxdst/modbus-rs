use crate::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadDiscreteInputsRequest {
    pub address: u16,
    pub length: u16,
}

impl Encodable for ReadDiscreteInputsRequest {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u16(self.address);
        encoder.write_u16(self.length);
        return Ok(());
    }
}

impl Decodable<Self> for ReadDiscreteInputsRequest {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        return Ok(Self {
            address: decoder.read_u16()?,
            length: decoder.read_u16()?,
        });
    }
}
