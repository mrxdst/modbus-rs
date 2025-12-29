use bytes::Buf;
use std::{
    io::{Cursor, Read},
    num::TryFromIntError,
};

#[derive(PartialEq, Debug)]
pub enum EncodeError {
    Overflow,
}

impl From<TryFromIntError> for EncodeError {
    fn from(_: TryFromIntError) -> Self {
        Self::Overflow
    }
}

pub type EncodeResult = Result<(), EncodeError>;

pub trait Encodable {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult;

    fn encode_to_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        Encoder::encode(self)
    }
}

pub struct Encoder {
    buffer: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(16),
        }
    }

    #[allow(unused)]
    pub fn position(&self) -> usize {
        self.buffer.len()
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buffer.push(value);
    }

    pub fn write_u16(&mut self, value: u16) {
        self.buffer.extend(value.to_be_bytes());
    }

    pub fn write_bools(&mut self, values: &[bool]) {
        let byte_length = values.len().div_ceil(8);
        self.buffer.reserve(byte_length);
        for i in 0..byte_length {
            let mut byte = 0;
            for i2 in 0..8 {
                if values.get(i * 8 + i2).copied().unwrap_or_default() {
                    byte |= 1 << i2;
                }
            }
            self.write_u8(byte);
        }
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.buffer.extend(value);
    }

    pub fn write_registers(&mut self, value: &[u16]) {
        self.buffer.extend(value.iter().flat_map(|v| v.to_be_bytes()));
    }

    pub fn write_type<T>(&mut self, value: &T) -> EncodeResult
    where
        T: Encodable + ?Sized,
    {
        value.encode(self)
    }

    pub fn finish(self) -> Vec<u8> {
        self.buffer
    }

    pub fn encode<T>(value: &T) -> Result<Vec<u8>, EncodeError>
    where
        T: Encodable + ?Sized,
    {
        let mut encoder = Self::new();
        encoder.write_type(value)?;
        Ok(encoder.finish())
    }
}

#[derive(PartialEq, Debug)]
pub enum DecodeError {
    MissingData,
    InvalidData(&'static str),
}

pub type DecodeResult<T> = Result<T, DecodeError>;

pub trait Decodable<T> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<T>;

    fn decode_from_bytes(buffer: &[u8]) -> DecodeResult<T>
    where
        T: Decodable<T>,
    {
        Decoder::decode(buffer)
    }
}

pub struct Decoder<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> Decoder<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            cursor: Cursor::new(buffer),
        }
    }

    pub fn position(&self) -> usize {
        self.cursor.position() as usize
    }

    #[allow(unused)]
    pub fn remaining(&self) -> usize {
        self.cursor.remaining()
    }

    pub fn read_u8(&mut self) -> DecodeResult<u8> {
        if self.cursor.remaining() < 1 {
            return Err(DecodeError::MissingData);
        }
        Ok(self.cursor.get_u8())
    }

    pub fn read_u16(&mut self) -> DecodeResult<u16> {
        if self.cursor.remaining() < 2 {
            return Err(DecodeError::MissingData);
        }
        Ok(self.cursor.get_u16())
    }

    pub fn read_bools(&mut self, length: usize) -> DecodeResult<Vec<bool>> {
        let byte_length = length.div_ceil(8);
        let mut values = Vec::with_capacity(length);
        for _ in 0..byte_length {
            let byte = self.read_u8()?;
            for i2 in 0..8 {
                if values.len() == length {
                    break;
                }
                values.push((byte & (1 << i2)) > 0);
            }
        }
        Ok(values)
    }

    pub fn read_bytes(&mut self, length: usize) -> DecodeResult<Vec<u8>> {
        if self.cursor.remaining() < length {
            return Err(DecodeError::MissingData);
        }
        let mut bytes = vec![0u8; length];
        self.cursor.read_exact(&mut bytes).unwrap();
        Ok(bytes)
    }

    pub fn read_registers(&mut self, length: usize) -> DecodeResult<Vec<u16>> {
        if self.cursor.remaining() < length * 2 {
            return Err(DecodeError::MissingData);
        }
        let mut registers = Vec::with_capacity(length);
        for _ in 0..length {
            registers.push(self.cursor.get_u16());
        }
        Ok(registers)
    }

    pub fn read_type<T>(&mut self) -> DecodeResult<T>
    where
        T: Decodable<T>,
    {
        T::decode(self)
    }

    pub fn decode<T>(buffer: &'a [u8]) -> DecodeResult<T>
    where
        T: Decodable<T>,
    {
        let mut decoder = Self::new(buffer);
        let value: T = decoder.read_type()?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode() {
        let mut encoder = Encoder::new();
        encoder.write_u8(0xAA);
        encoder.write_u16(0xBBCC);
        encoder.write_bytes(&[1, 2, 3]);
        encoder.write_registers(&[300, 301, 302]);

        assert_eq!(encoder.position(), 12);

        let bytes = encoder.finish();

        assert_eq!(bytes.len(), 12);

        let mut decoder = Decoder::new(&bytes);

        assert_eq!(decoder.position(), 0);
        assert_eq!(decoder.remaining(), 12);

        assert_eq!(decoder.read_u8(), Ok(0xAA));
        assert_eq!(decoder.read_u16(), Ok(0xBBCC));
        assert_eq!(decoder.read_bytes(3), Ok(vec![1, 2, 3]));
        assert_eq!(decoder.read_registers(3), Ok(vec![300, 301, 302]));

        assert_eq!(decoder.position(), 12);
        assert_eq!(decoder.remaining(), 0);
    }
}
