use std::{borrow::Cow, collections::HashMap};

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ModbusEncapsulatedInterfaceType {
    ReadDeviceIdentification = 14,
    Unknown(u8),
}

impl From<u8> for ModbusEncapsulatedInterfaceType {
    fn from(value: u8) -> Self {
        match value {
            14 => Self::ReadDeviceIdentification,
            _ => Self::Unknown(value),
        }
    }
}

impl From<ModbusEncapsulatedInterfaceType> for u8 {
    fn from(value: ModbusEncapsulatedInterfaceType) -> Self {
        match value {
            ModbusEncapsulatedInterfaceType::ReadDeviceIdentification => 14,
            ModbusEncapsulatedInterfaceType::Unknown(value) => value,
        }
    }
}

impl PartialEq for ModbusEncapsulatedInterfaceType {
    fn eq(&self, other: &Self) -> bool {
        u8::from(*self) == u8::from(*other)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ReadDeviceIdentificationIdCode {
    Basic = 1,
    Regular = 2,
    Extended = 3,
    Individual = 4,
    Unknown(u8),
}

impl From<u8> for ReadDeviceIdentificationIdCode {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Basic,
            2 => Self::Regular,
            3 => Self::Extended,
            4 => Self::Individual,
            _ => Self::Unknown(value),
        }
    }
}

impl From<ReadDeviceIdentificationIdCode> for u8 {
    fn from(value: ReadDeviceIdentificationIdCode) -> Self {
        match value {
            ReadDeviceIdentificationIdCode::Basic => 1,
            ReadDeviceIdentificationIdCode::Regular => 2,
            ReadDeviceIdentificationIdCode::Extended => 3,
            ReadDeviceIdentificationIdCode::Individual => 4,
            ReadDeviceIdentificationIdCode::Unknown(value) => value,
        }
    }
}

impl PartialEq for ReadDeviceIdentificationIdCode {
    fn eq(&self, other: &Self) -> bool {
        u8::from(*self) == u8::from(*other)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ReadDeviceIdentificationConformityLevel {
    BasicStream = 0x01,
    RegularStream = 0x02,
    ExtendedStream = 0x03,
    BasicStreamAndIndividual = 0x81,
    RegularStreamAndIndividual = 0x82,
    ExtendedStreamAndIndividual = 0x83,
    Unknown(u8),
}

impl From<u8> for ReadDeviceIdentificationConformityLevel {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::BasicStream,
            0x02 => Self::RegularStream,
            0x03 => Self::ExtendedStream,
            0x81 => Self::BasicStreamAndIndividual,
            0x82 => Self::RegularStreamAndIndividual,
            0x83 => Self::ExtendedStreamAndIndividual,
            _ => Self::Unknown(value),
        }
    }
}

impl From<ReadDeviceIdentificationConformityLevel> for u8 {
    fn from(value: ReadDeviceIdentificationConformityLevel) -> Self {
        match value {
            ReadDeviceIdentificationConformityLevel::BasicStream => 0x01,
            ReadDeviceIdentificationConformityLevel::RegularStream => 0x02,
            ReadDeviceIdentificationConformityLevel::ExtendedStream => 0x03,
            ReadDeviceIdentificationConformityLevel::BasicStreamAndIndividual => 0x81,
            ReadDeviceIdentificationConformityLevel::RegularStreamAndIndividual => 0x82,
            ReadDeviceIdentificationConformityLevel::ExtendedStreamAndIndividual => 0x83,
            ReadDeviceIdentificationConformityLevel::Unknown(value) => value,
        }
    }
}

impl PartialEq for ReadDeviceIdentificationConformityLevel {
    fn eq(&self, other: &Self) -> bool {
        u8::from(*self) == u8::from(*other)
    }
}

/// Data structure used when reading identification and additional information from a device.
#[derive(PartialEq, Debug, Clone)]
pub struct DeviceIdentification<'a> {
    pub vendor_name: Cow<'a, str>,
    pub product_code: Cow<'a, str>,
    pub major_minor_revision: Cow<'a, str>,
    pub vendor_url: Option<Cow<'a, str>>,
    pub product_name: Option<Cow<'a, str>>,
    pub model_name: Option<Cow<'a, str>>,
    pub user_application_name: Option<Cow<'a, str>>,
    /// Private objects may be optionally defined.
    /// The range [0x80 – 0xFF] is product dependant.
    pub objects: HashMap<u8, Cow<'a, [u8]>>,
}
