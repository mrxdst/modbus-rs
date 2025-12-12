use std::{
    borrow::Cow,
    collections::HashMap,
    error::Error,
    fmt::Display,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
};

use tokio::{net::TcpStream, task::AbortHandle};
use tokio::{
    sync::{oneshot, Mutex},
    task::JoinHandle,
};

use crate::{
    connection::*, consts::*, encoding::*, function_code::FunctionCode, message::Message, messages::*, modbus_encapsulated_interface::*,
    modbus_exception::ModbusException,
};

/// Errors returned by the [`ModbusTCPClient`].
#[derive(Debug, Clone)]
pub enum ModbusError {
    /// Represent an IO error.
    IO(Arc<tokio::io::Error>),
    /// Some arguments provided to the function are out of range.
    /// Commonly the combination of address + length is outside the allowed range.
    /// The request was never sent to the server.
    ArgumentsOutOfRange(String),
    /// Indicates that the response received from the server is not a valid response.
    InvalidResponse(String),
    /// Exception code reported by the server.
    ModbusException(ModbusException),
}

impl Display for ModbusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModbusError::IO(err) => write!(f, "{err}"),
            ModbusError::ArgumentsOutOfRange(err) => write!(f, "Argument out of range: {err}"),
            ModbusError::InvalidResponse(err) => write!(f, "Invalid response: {err}"),
            ModbusError::ModbusException(ex) => write!(f, "{ex:?}"),
        }
    }
}

impl Error for ModbusError {}

type ResponseResult = Result<Message, ModbusError>;
type ResponseMap = Arc<Mutex<HashMap<u16, oneshot::Sender<ResponseResult>>>>;

pub struct ModbusTCPClient {
    connection: Arc<Connection>,
    transaction_id: AtomicU16,
    response_map: ResponseMap,
    abort_handle: AbortHandle,
}

impl ModbusTCPClient {
    pub fn new(stream: TcpStream) -> (Self, JoinHandle<Result<(), ModbusError>>) {
        let connection = Arc::new(Connection::new(stream));
        let response_map = Arc::new(Mutex::new(HashMap::new()));

        let join_handle = tokio::spawn(Self::receive_response(connection.clone(), response_map.clone()));

        let client = Self {
            connection,
            transaction_id: AtomicU16::default(),
            response_map,
            abort_handle: join_handle.abort_handle(),
        };

        (client, join_handle)
    }

    pub async fn read_coils(&self, unit_id: u8, address: u16, length: u16) -> Result<Vec<bool>, ModbusError> {
        validate_input(address, length as usize, READ_COILS_MAX_LEN)?;
        let req = ReadCoilsRequest { address, length };
        let req_body = req.encode_to_bytes().expect("Couldn't encode request");
        let result = self.send_request(unit_id, FunctionCode::ReadCoils, req_body).await?;
        let res = ReadCoilsResponse::decode_from_bytes(&result).map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;
        Ok(res.values.into())
    }

    pub async fn read_discrete_inputs(&self, unit_id: u8, address: u16, length: u16) -> Result<Vec<bool>, ModbusError> {
        validate_input(address, length as usize, READ_DISCRETE_INPUTS_MAX_LEN)?;
        let req = ReadDiscreteInputsRequest { address, length };
        let req_body = req.encode_to_bytes().expect("Couldn't encode request");
        let result = self.send_request(unit_id, FunctionCode::ReadDiscreteInputs, req_body).await?;
        let res = ReadDiscreteInputsResponse::decode_from_bytes(&result).map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;
        Ok(res.values.into())
    }

    pub async fn read_input_registers(&self, unit_id: u8, address: u16, length: u16) -> Result<Vec<u16>, ModbusError> {
        validate_input(address, length as usize, READ_INPUT_REGISTERS_MAX_LEN)?;
        let req = ReadInputRegistersRequest { address, length };
        let req_body = req.encode_to_bytes().expect("Couldn't encode request");
        let result = self.send_request(unit_id, FunctionCode::ReadInputRegisters, req_body).await?;
        let res = ReadInputRegistersResponse::decode_from_bytes(&result).map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;
        Ok(res.values.into())
    }

    pub async fn read_holding_registers(&self, unit_id: u8, address: u16, length: u16) -> Result<Vec<u16>, ModbusError> {
        validate_input(address, length as usize, READ_HOLDING_REGISTERS_MAX_LEN)?;
        let req = ReadHoldingRegistersRequest { address, length };
        let req_body = req.encode_to_bytes().expect("Couldn't encode request");
        let result = self.send_request(unit_id, FunctionCode::ReadHoldingRegisters, req_body).await?;
        let res =
            ReadHoldingRegistersResponse::decode_from_bytes(&result).map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;
        Ok(res.values.into())
    }

    pub async fn write_single_coils(&self, unit_id: u8, address: u16, value: bool) -> Result<(), ModbusError> {
        let req = WriteSingleCoilRequest { address, value };
        let req_body = req.encode_to_bytes().expect("Couldn't encode request");
        let result = self.send_request(unit_id, FunctionCode::WriteSingleCoil, req_body).await?;
        let res = WriteSingleCoilResponse::decode_from_bytes(&result).map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;
        if res.address == req.address && res.value == req.value {
            Ok(())
        } else {
            Err(ModbusError::InvalidResponse("Malformed response".to_string()))
        }
    }

    pub async fn write_single_holding_register(&self, unit_id: u8, address: u16, value: u16) -> Result<(), ModbusError> {
        let req = WriteSingleHoldingRegisterRequest { address, value };
        let req_body = req.encode_to_bytes().expect("Couldn't encode request");
        let result = self.send_request(unit_id, FunctionCode::WriteSingleHoldingRegister, req_body).await?;
        let res = WriteSingleHoldingRegisterResponse::decode_from_bytes(&result)
            .map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;
        if res.address == req.address && res.value == req.value {
            Ok(())
        } else {
            Err(ModbusError::InvalidResponse("Malformed response".to_string()))
        }
    }

    pub async fn write_multiple_coils(&self, unit_id: u8, address: u16, values: &[bool]) -> Result<(), ModbusError> {
        validate_input(address, values.len(), WRITE_MULTIPLE_COILS_MAX_LEN)?;
        let req = WriteMultipleCoilsRequest {
            address,
            values: values.into(),
        };
        let req_body = req.encode_to_bytes().expect("Couldn't encode request");
        let result = self.send_request(unit_id, FunctionCode::WriteMultipleCoils, req_body).await?;
        let res = WriteMultipleCoilsResponse::decode_from_bytes(&result).map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;
        if res.address == req.address && res.length as usize == req.values.len() {
            Ok(())
        } else {
            Err(ModbusError::InvalidResponse("Malformed response".to_string()))
        }
    }

    pub async fn write_multiple_holding_registers(&self, unit_id: u8, address: u16, values: &[u16]) -> Result<(), ModbusError> {
        validate_input(address, values.len(), WRITE_MULTIPLE_HOLDING_REGISTERS_MAX_LEN)?;
        let req = WriteMultipleHoldingRegistersRequest {
            address,
            values: values.into(),
        };
        let req_body = req.encode_to_bytes().expect("Couldn't encode request");
        let result = self.send_request(unit_id, FunctionCode::WriteMultipleHoldingRegisters, req_body).await?;
        let res = WriteMultipleHoldingRegistersResponse::decode_from_bytes(&result)
            .map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;
        if res.address == req.address && res.length as usize == req.values.len() {
            Ok(())
        } else {
            Err(ModbusError::InvalidResponse("Malformed response".to_string()))
        }
    }

    pub async fn mask_write_holding_registers(&self, unit_id: u8, address: u16, and_mask: u16, or_mask: u16) -> Result<(), ModbusError> {
        let req = MaskWriteHoldingRegisterRequest { address, and_mask, or_mask };
        let req_body = req.encode_to_bytes().expect("Couldn't encode request");
        let result = self.send_request(unit_id, FunctionCode::MaskWriteHoldingRegister, req_body).await?;
        let res =
            MaskWriteHoldingRegisterResponse::decode_from_bytes(&result).map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;
        if res.address == req.address && res.and_mask == req.and_mask && res.or_mask == req.or_mask {
            Ok(())
        } else {
            Err(ModbusError::InvalidResponse("Malformed response".to_string()))
        }
    }

    pub async fn modbus_encapsulated_interface(&self, unit_id: u8, interface_type: u8, data: &[u8]) -> Result<Vec<u8>, ModbusError> {
        let req = ModbusEncapsulatedInterfaceRequest {
            kind: ModbusEncapsulatedInterfaceType::Unknown(interface_type),
            data: data.into(),
        };

        let res_body = self
            .send_request(unit_id, FunctionCode::ModbusEncapsulatedInterface, req.encode_to_bytes().unwrap())
            .await?;
        let res = ModbusEncapsulatedInterfaceResponse::decode_from_bytes(&res_body)
            .map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;

        if res.kind != req.kind {
            return Err(ModbusError::InvalidResponse("Interface type mismatch".to_string()));
        }

        Ok(res.data.into())
    }

    pub async fn read_device_identification(&self, unit_id: u8) -> Result<DeviceIdentification<'_>, ModbusError> {
        let mut more_follows = true;
        let mut next_object_id = 0u8;

        let mut result = DeviceIdentification {
            vendor_name: "".into(),
            product_code: "".into(),
            major_minor_revision: "".into(),
            model_name: None,
            product_name: None,
            user_application_name: None,
            vendor_url: None,
            objects: HashMap::new(),
        };

        for _ in 0..0xFFu8 {
            if !more_follows {
                break;
            }

            let req = ReadDeviceIdentificationRequest {
                object_id: next_object_id,
                device_id_code: ReadDeviceIdentificationIdCode::Extended,
            };

            let res_body = self
                .modbus_encapsulated_interface(
                    unit_id,
                    ModbusEncapsulatedInterfaceType::ReadDeviceIdentification.into(),
                    &req.encode_to_bytes().unwrap(),
                )
                .await?;
            let res = ReadDeviceIdentificationResponse::decode_from_bytes(&res_body)
                .map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;

            more_follows = res.more_follows;
            next_object_id = res.next_object_id;

            for (id, data) in res.objects {
                let str_data = || -> Cow<str> { String::from_utf8_lossy(&data).to_string().into() };
                match id {
                    0 => result.vendor_name = str_data(),
                    1 => result.product_code = str_data(),
                    2 => result.major_minor_revision = str_data(),
                    3 => result.vendor_url = Some(str_data()),
                    4 => result.product_name = Some(str_data()),
                    5 => result.model_name = Some(str_data()),
                    6 => result.user_application_name = Some(str_data()),
                    _ => {
                        result.objects.insert(id, data);
                    }
                }
            }
        }

        Ok(result)
    }

    async fn send_request(&self, unit_id: u8, function_code: FunctionCode, body: Vec<u8>) -> Result<Vec<u8>, ModbusError> {
        let transaction_id = self.transaction_id.fetch_add(1, Ordering::Relaxed);

        let msg = Message {
            protocol_id: 0,
            transaction_id,
            function_code,
            unit_id,
            body,
        };

        let (sender, receiver) = oneshot::channel::<ResponseResult>();

        {
            let mut map = self.response_map.lock().await;
            map.insert(transaction_id, sender);
        }

        match self.connection.write_message(&msg).await {
            Ok(_) => {}
            Err(WriteError::IO(e)) => return Err(ModbusError::IO(e.into())),
            Err(WriteError::Encode(_)) => return Err(ModbusError::ArgumentsOutOfRange("Error encoding message".to_string())),
        }

        let res_msg = match receiver.await.unwrap() {
            Ok(msg) => msg,
            Err(error) => return Err(error),
        };

        if res_msg.protocol_id != msg.protocol_id {
            return Err(ModbusError::InvalidResponse("Protocol id mismatch".to_string()));
        }
        if res_msg.unit_id != msg.unit_id {
            return Err(ModbusError::InvalidResponse("Unit id mismatch".to_string()));
        }
        if let FunctionCode::Error(_) = res_msg.function_code {
            let ex_res =
                ExceptionMessage::decode_from_bytes(&res_msg.body).map_err(|_| ModbusError::InvalidResponse("Malformed response".to_string()))?;
            return Err(ModbusError::ModbusException(ex_res.code));
        }
        if res_msg.function_code != msg.function_code {
            return Err(ModbusError::InvalidResponse("Function code mismatch".to_string()));
        }

        Ok(res_msg.body)
    }

    async fn receive_response(connection: Arc<Connection>, response_map: ResponseMap) -> Result<(), ModbusError> {
        loop {
            let msg = match connection.read_message().await {
                Ok(Some(msg)) => msg,
                Ok(None) => return Ok(()),
                Err(error) => {
                    let error = match error {
                        ReadError::IO(error) => ModbusError::IO(error.into()),
                        ReadError::Decode(_) => ModbusError::InvalidResponse("The server sent invalid data".into()),
                    };
                    let mut response_map = response_map.lock().await;
                    for (_, sender) in response_map.drain() {
                        _ = sender.send(Err(error.clone()));
                    }
                    return Err(error);
                }
            };

            let sender = response_map.lock().await.remove(&msg.transaction_id);
            match sender {
                None => return Err(ModbusError::InvalidResponse("The server sent an unexpected response".into())),
                Some(sender) => _ = sender.send(Ok(msg)),
            }
        }
    }
}

impl Drop for ModbusTCPClient {
    fn drop(&mut self) {
        self.abort_handle.abort();
    }
}

fn validate_input(address: u16, length: usize, max_length: u16) -> Result<(), ModbusError> {
    if length == 0 || length > max_length as usize {
        return Err(ModbusError::ArgumentsOutOfRange(format!(
            "Length exceeds maximum allowed length {max_length}"
        )));
    }
    u16::checked_add(address, (length - 1) as u16)
        .ok_or(ModbusError::ArgumentsOutOfRange("Address + length exceeds device address space".to_string()))?;
    Ok(())
}
