use crate::{encoding::*, modbus_encapsulated_interface::*};

#[derive(PartialEq, Debug)]
pub struct ReadDeviceIdentificationRequest {
    pub device_id_code: ReadDeviceIdentificationIdCode,
    pub object_id: u8,
}

impl Encodable for ReadDeviceIdentificationRequest {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8(self.device_id_code.into());
        encoder.write_u8(self.object_id);
        return Ok(());
    }
}

impl Decodable<Self> for ReadDeviceIdentificationRequest {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        return Ok(Self {
            device_id_code: decoder.read_u8()?.into(),
            object_id: decoder.read_u8()?,
        });
    }
}
