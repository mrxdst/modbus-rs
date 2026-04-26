use tokio::io::{AsyncRead, AsyncReadExt};

use crate::connection::ReadError;
use crate::function_code::FunctionCode;

use super::encoding::*;

pub const MSG_MAX_LENGTH: usize = 260;

#[derive(PartialEq, Debug)]
pub struct Message {
    pub transaction_id: u16,
    pub protocol_id: u16,
    pub unit_id: u8,
    pub function_code: FunctionCode,
    pub body: Vec<u8>,
}

impl Encodable for Message {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u16(self.transaction_id);
        encoder.write_u16(self.protocol_id);
        encoder.write_u16((self.body.len() + 2).try_into()?);
        encoder.write_u8(self.unit_id);
        encoder.write_u8(self.function_code.into());
        encoder.write_bytes(&self.body);
        Ok(())
    }
}

impl Message {
    pub async fn read<R>(reader: &mut R) -> Result<Message, ReadError> 
    where
        R: AsyncRead + Unpin + ?Sized
    {
        let mut header = [0; 8];

        reader.read_exact(&mut header).await?;

        let mut decoder = Decoder::new(&header);

        let transaction_id = decoder.read_u16()?;
        let protocol_id = decoder.read_u16()?;
        let byte_length = decoder.read_u16()?;

        if byte_length as usize > MSG_MAX_LENGTH - 6 {
            return Err(DecodeError::InvalidData("Byte length to large").into());
        }

        let unit_id = decoder.read_u8()?;
        let function_code = decoder.read_u8()?.into();

        let mut body = vec![0; (byte_length - 2).into()];
        reader.read_exact(&mut body).await?;

        Ok(Message {
            transaction_id,
            protocol_id,
            unit_id,
            function_code,
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::function_code::FunctionCode;

    use super::*;

    #[tokio::test]
    async fn encode_decode() {
        let msg = Message {
            transaction_id: 1,
            protocol_id: 2,
            unit_id: 3,
            function_code: FunctionCode::ReadInputRegisters,
            body: vec![5, 6, 7],
        };

        let bytes = msg.encode_to_bytes().unwrap();

        assert_eq!(bytes.len(), 11);

        let mut cur = Cursor::new(bytes);
        
        let decoded_msg = Message::read(&mut cur).await.unwrap();

        assert_eq!(msg, decoded_msg);
    }
}
