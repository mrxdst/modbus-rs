use crate::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadCoilsRequest {
    pub address: u16,
    pub length: u16,
}

impl Encodable for ReadCoilsRequest {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u16(self.address);
        encoder.write_u16(self.length);
        Ok(())
    }
}

impl Decodable<Self> for ReadCoilsRequest {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self {
            address: decoder.read_u16()?,
            length: decoder.read_u16()?,
        })
    }
}
