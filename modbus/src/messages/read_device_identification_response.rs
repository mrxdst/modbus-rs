use std::{borrow::Cow, collections::HashMap};

use crate::{encoding::*, modbus_encapsulated_interface::*};

#[derive(PartialEq, Debug)]
pub struct ReadDeviceIdentificationResponse<'a> {
    pub device_id_code: ReadDeviceIdentificationIdCode,
    pub conformity_level: ReadDeviceIdentificationConformityLevel,
    pub more_follows: bool,
    pub next_object_id: u8,
    pub objects: HashMap<u8, Cow<'a, [u8]>>,
}

impl<'a> Encodable for ReadDeviceIdentificationResponse<'a> {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8(self.device_id_code.into());
        encoder.write_u8(self.conformity_level.into());
        encoder.write_u8(if self.more_follows { 0xFF } else { 0x00 });
        encoder.write_u8(self.next_object_id);
        encoder.write_u8(self.objects.len().try_into()?);
        for (id, data) in self.objects.iter() {
            encoder.write_u8(*id);
            encoder.write_u8(data.len().try_into()?);
            encoder.write_bytes(data);
        }
        return Ok(());
    }
}

impl<'a> Decodable<Self> for ReadDeviceIdentificationResponse<'a> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        let device_id_code = decoder.read_u8()?.into();
        let conformity_level = decoder.read_u8()?.into();
        let more_follows = decoder.read_u8()? != 0;
        let next_object_id = decoder.read_u8()?;
        let length = decoder.read_u8()?;
        let mut objects = HashMap::with_capacity(length.into());
        for _ in 0..length {
            let id = decoder.read_u8()?;
            let length = decoder.read_u8()?;
            let data = decoder.read_bytes(length.into())?;
            objects.insert(id, data.into());
        }
        return Ok(Self {
            device_id_code,
            conformity_level,
            more_follows,
            next_object_id,
            objects,
        });
    }
}
