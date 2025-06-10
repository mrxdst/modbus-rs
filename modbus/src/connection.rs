use bytes::{Buf, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
};

use crate::{encoding::*, message::Message};

pub struct Connection {
    reader: Mutex<OwnedReadHalf>,
    writer: Mutex<OwnedWriteHalf>,
    read_buffer: Mutex<BytesMut>,
}

#[derive(Debug)]
pub enum ReadError {
    #[allow(unused)]
    IO(tokio::io::Error),
    #[allow(unused)]
    Decode(DecodeError),
}

#[derive(Debug)]
pub enum WriteError {
    #[allow(unused)]
    IO(tokio::io::Error),
    Encode(EncodeError),
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
            read_buffer: Mutex::new(BytesMut::with_capacity(32)),
        }
    }

    pub async fn read_message(&self) -> Result<Option<Message>, ReadError> {
        loop {
            let mut reader = self.reader.lock().await;
            let mut read_buffer = self.read_buffer.lock().await;

            loop {
                let mut decoder = Decoder::new(&read_buffer);
                let msg = decoder.read_type();

                match msg {
                    Ok(msg) => {
                        let pos = decoder.position();
                        read_buffer.advance(pos);
                        return Ok(Some(msg));
                    }
                    Err(err) => {
                        match err {
                            DecodeError::InvalidData(_) => return Err(ReadError::Decode(err)),
                            DecodeError::MissingData => break, // wait for more data
                        }
                    }
                }
            }

            let bytes_read = reader.read_buf(&mut *read_buffer).await.map_err(|e| ReadError::IO(e))?;

            if bytes_read == 0 {
                _ = self.writer.lock().await.shutdown().await;
                return Ok(None);
            }
        }
    }

    pub async fn write_message(&self, msg: &Message) -> Result<(), WriteError> {
        let bytes = msg.encode_to_bytes().map_err(|e| WriteError::Encode(e))?;

        let mut writer = self.writer.lock().await;
        writer.write_all(&bytes).await.map_err(|e| WriteError::IO(e))?;

        Ok(())
    }

    #[allow(unused)]
    pub async fn shutdown(&self) -> Result<(), std::io::Error> {
        self.writer.lock().await.shutdown().await
    }
}
