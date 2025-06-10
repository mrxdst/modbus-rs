use std::{error::Error, fmt::Display};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AddressKind {
    Coil = 0,
    DiscreteInput = 1,
    InputRegister = 3,
    HoldingRegister = 4,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct Address {
    pub kind: AddressKind,
    pub index: u16,
}

#[derive(PartialEq, Clone, Debug)]
pub struct ParseAddressError(String);

impl From<&str> for ParseAddressError {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl Display for ParseAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for ParseAddressError {}

impl Address {
    pub fn parse(value: &str, offset: i32) -> Result<Self, ParseAddressError> {
        let mut iter = value.chars();

        let kind = match iter.next() {
            Some('0') => AddressKind::Coil,
            Some('1') => AddressKind::DiscreteInput,
            Some('3') => AddressKind::InputRegister,
            Some('4') => AddressKind::HoldingRegister,
            Some(_) => return Err("Address must start with 0, 1, 3 or 4.".into()),
            None => return Err("Empty value".into()),
        };

        let index = iter.by_ref().take(5).collect::<String>();
        if index.is_empty() {
            return Err("Address must be at least 2 digits long".into());
        }
        if iter.next().is_some() {
            return Err("Address must be at most 6 digits".into());
        }

        let index = index.parse::<u32>().map_err(|_| "Address out of range")?;
        let index: i32 = index.try_into().map_err(|_| "Address out of range")?;
        let index = index + offset;
        let index: u16 = index.try_into().map_err(|_| "Address out of range")?;

        Ok(Self { kind, index })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address() {
        assert!(Address::parse("", 0).is_err());
        assert!(Address::parse("1", 0).is_err());
        assert_eq!(
            Address::parse("12", 0),
            Ok(Address {
                kind: AddressKind::DiscreteInput,
                index: 2
            })
        );
        assert_eq!(
            Address::parse("123456", 0),
            Ok(Address {
                kind: AddressKind::DiscreteInput,
                index: 23456
            })
        );
        assert!(Address::parse("1234567", 0).is_err());
        assert!(Address::parse("22", 0).is_err());
        assert_eq!(
            Address::parse("11", -1),
            Ok(Address {
                kind: AddressKind::DiscreteInput,
                index: 0
            })
        );
        assert_eq!(
            Address::parse("110", 800),
            Ok(Address {
                kind: AddressKind::DiscreteInput,
                index: 810
            })
        );
        assert!(Address::parse("110", 80000).is_err());
        assert!(Address::parse("110", -11).is_err());
    }
}
