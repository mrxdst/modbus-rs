use crate::encoding::*;

#[derive(PartialEq, Debug)]
pub struct WriteSingleCoilRequest {
    pub address: u16,
    pub value: bool,
}

impl Encodable for WriteSingleCoilRequest {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u16(self.address);
        encoder.write_u16(if self.value { 0xFF00 } else { 0 });
        Ok(())
    }
}

impl Decodable<Self> for WriteSingleCoilRequest {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self {
            address: decoder.read_u16()?,
            value: decoder.read_u16()? != 0,
        })
    }
}
