use std::ops::DerefMut;

use thiserror::Error;
use tokio::{
    io::AsyncWriteExt,
    net::{
        TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}
    },
    sync::Mutex,
};

use crate::{encoding::*, message::Message};

pub struct Connection {
    reader: Mutex<OwnedReadHalf>,
    writer: Mutex<OwnedWriteHalf>,
}

#[derive(Error, Debug)]
pub enum ReadError {
    #[error(transparent)]
    IO(#[from] tokio::io::Error),
    #[error(transparent)]
    Decode(#[from] DecodeError),
}

#[derive(Error, Debug)]
pub enum WriteError {
    #[error(transparent)]
    IO(#[from] tokio::io::Error),
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
        }
    }

    pub async fn read_message(&self) -> Result<Option<Message>, ReadError> {
        let mut reader = self.reader.lock().await;

        let mut t = [0];
        let len = reader.peek(&mut t).await?;
        if len == 0 { return Ok(None); }

        Ok(Some(Message::read(reader.deref_mut()).await?))
    }

    pub async fn write_message(&self, msg: &Message) -> Result<(), WriteError> {
        let bytes = msg.encode_to_bytes().map_err(WriteError::Encode)?;

        let mut writer = self.writer.lock().await;

        writer.write_all(&bytes).await.map_err(WriteError::IO)?;

        Ok(())
    }

    #[allow(unused)]
    pub async fn shutdown(&self) -> Result<(), std::io::Error> {
        self.writer.lock().await.shutdown().await
    }
}
