use crate::encoding::*;

#[derive(PartialEq, Debug)]
pub struct MaskWriteHoldingRegisterRequest {
    pub address: u16,
    pub and_mask: u16,
    pub or_mask: u16,
}

impl Encodable for MaskWriteHoldingRegisterRequest {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u16(self.address);
        encoder.write_u16(self.and_mask);
        encoder.write_u16(self.or_mask);
        return Ok(());
    }
}

impl Decodable<Self> for MaskWriteHoldingRegisterRequest {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        return Ok(Self {
            address: decoder.read_u16()?,
            and_mask: decoder.read_u16()?,
            or_mask: decoder.read_u16()?,
        });
    }
}
