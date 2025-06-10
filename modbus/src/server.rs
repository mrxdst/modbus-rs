use std::{borrow::Cow, collections::HashMap, future::Future, marker::PhantomData, net::SocketAddr, sync::Arc};

use tokio::{
    net::TcpListener,
    sync::{Mutex, Semaphore},
    task::JoinHandle,
};

use crate::{
    connection::Connection,
    consts::*,
    encoding::{Decodable, Encodable},
    function_code::FunctionCode,
    message::{Message, MSG_MAX_LENGTH},
    messages::*,
    modbus_encapsulated_interface::*,
    modbus_exception::ModbusException,
};

/**
 * Handlers to be implemented by servers.
 * Default implementation is to respond to requests with [`ModbusException::IllegalFunction`].
 */
pub trait ModbusTCPServerHandler: Send + Sync + 'static {
    /// Whether to accept a new connection. Default is to always accept.
    #[allow(unused_variables)]
    fn accept_connection(&self, addr: SocketAddr) -> impl Future<Output = bool> + Send {
        async { true }
    }
    /// The maximum number of concurrent connections.
    fn max_concurrent_connections(&self) -> usize {
        100
    }
    /// The maximum number of concurrent requests per connection.
    fn max_concurrent_requests(&self) -> usize {
        10
    }
    #[allow(unused_variables)]
    fn disconnected(&self, addr: SocketAddr) -> impl Future<Output = ()> + Send {
        async {}
    }
    #[allow(unused_variables)]
    fn handle_read_coils(
        &self,
        addr: SocketAddr,
        unit_id: u8,
        address: u16,
        length: u16,
    ) -> impl Future<Output = Result<Cow<[bool]>, ModbusException>> + Send {
        async { Err(ModbusException::IllegalFunction) }
    }
    #[allow(unused_variables)]
    fn handle_read_discrete_inputs(
        &self,
        addr: SocketAddr,
        unit_id: u8,
        address: u16,
        length: u16,
    ) -> impl Future<Output = Result<Cow<[bool]>, ModbusException>> + Send {
        async { Err(ModbusException::IllegalFunction) }
    }
    #[allow(unused_variables)]
    fn handle_read_input_registers(
        &self,
        addr: SocketAddr,
        unit_id: u8,
        address: u16,
        length: u16,
    ) -> impl Future<Output = Result<Cow<[u16]>, ModbusException>> + Send {
        async { Err(ModbusException::IllegalFunction) }
    }
    #[allow(unused_variables)]
    fn handle_read_holding_registers(
        &self,
        addr: SocketAddr,
        unit_id: u8,
        address: u16,
        length: u16,
    ) -> impl Future<Output = Result<Cow<[u16]>, ModbusException>> + Send {
        async { Err(ModbusException::IllegalFunction) }
    }
    #[allow(unused_variables)]
    fn handle_write_coils(
        &self,
        addr: SocketAddr,
        unit_id: u8,
        address: u16,
        values: &[bool],
    ) -> impl Future<Output = Result<(), ModbusException>> + Send {
        async { Err(ModbusException::IllegalFunction) }
    }
    #[allow(unused_variables)]
    fn handle_write_holding_registers(
        &self,
        addr: SocketAddr,
        unit_id: u8,
        address: u16,
        values: &[u16],
    ) -> impl Future<Output = Result<(), ModbusException>> + Send {
        async { Err(ModbusException::IllegalFunction) }
    }
    #[allow(unused_variables)]
    fn handle_read_device_identification(
        &self,
        addr: SocketAddr,
        unit_id: u8,
    ) -> impl Future<Output = Result<Cow<DeviceIdentification>, ModbusException>> + Send {
        async { Err(ModbusException::IllegalFunction) }
    }
    #[allow(unused_variables)]
    fn handle_modbus_encapsulated_interface(
        &self,
        addr: SocketAddr,
        unit_id: u8,
        interface_type: u8,
        data: &[u8],
    ) -> impl Future<Output = Result<Cow<[u8]>, ModbusException>> + Send {
        async { Err(ModbusException::IllegalFunction) }
    }
}

pub struct ModbusTCPServer<T> {
    phantom: PhantomData<T>,
}

impl<T> ModbusTCPServer<T>
where
    T: ModbusTCPServerHandler,
{
    pub fn run(listener: TcpListener, handler: Arc<T>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let connection_count = Arc::new(Mutex::new(0usize));

            loop {
                if let Ok((stream, addr)) = listener.accept().await {
                    let max_connections = handler.max_concurrent_connections();

                    let mut cnt = connection_count.lock().await;

                    if *cnt >= max_connections {
                        continue;
                    }

                    if !handler.accept_connection(addr).await {
                        continue;
                    }

                    *cnt = cnt.saturating_add(1);
                    drop(cnt);

                    let connection = Arc::new(Connection::new(stream));
                    let handler = handler.clone();
                    let connection_count = connection_count.clone();

                    tokio::spawn(async move {
                        Self::process(connection, addr, &handler).await;
                        handler.disconnected(addr).await;
                        let mut cnt = connection_count.lock().await;
                        *cnt = cnt.saturating_sub(1);
                    });
                }
            }
        })
    }

    async fn process(connection: Arc<Connection>, addr: SocketAddr, handler: &Arc<T>) {
        let limiter = Arc::new(Semaphore::new(match handler.max_concurrent_requests() {
            0 => Semaphore::MAX_PERMITS,
            v => v,
        }));

        while let Ok(Some(msg)) = connection.read_message().await {
            let permit = limiter.clone().acquire_owned().await.unwrap();
            let connection = connection.clone();
            let handler = handler.clone();
            tokio::spawn(async move {
                let result = Self::handle_request(&msg, addr, &handler).await;

                let res_msg = Message {
                    function_code: if result.is_err() {
                        msg.function_code.as_err()
                    } else {
                        msg.function_code
                    },
                    body: match result {
                        Ok(body) => body,
                        Err(code) => ExceptionMessage::from(code).encode_to_bytes().unwrap(),
                    },
                    ..msg
                };

                _ = connection.write_message(&res_msg).await; // Do something?

                drop(permit);
            });
        }
    }

    async fn handle_request(msg: &Message, addr: SocketAddr, handler: &Arc<T>) -> Result<Vec<u8>, ModbusException> {
        let bytes = match msg.function_code {
            FunctionCode::ReadCoils => {
                let req = ReadCoilsRequest::decode_from_bytes(&msg.body).map_err(|_| ModbusException::ServerDeviceFailure)?;
                Self::read_coils(addr, msg.unit_id, &req, handler).await?.encode_to_bytes()
            }
            FunctionCode::ReadDiscreteInputs => {
                let req = ReadDiscreteInputsRequest::decode_from_bytes(&msg.body).map_err(|_| ModbusException::ServerDeviceFailure)?;
                Self::read_discrete_inputs(addr, msg.unit_id, &req, handler).await?.encode_to_bytes()
            }
            FunctionCode::ReadInputRegisters => {
                let req = ReadInputRegistersRequest::decode_from_bytes(&msg.body).map_err(|_| ModbusException::ServerDeviceFailure)?;
                Self::read_input_registers(addr, msg.unit_id, &req, handler).await?.encode_to_bytes()
            }
            FunctionCode::ReadHoldingRegisters => {
                let req = ReadHoldingRegistersRequest::decode_from_bytes(&msg.body).map_err(|_| ModbusException::ServerDeviceFailure)?;
                Self::read_holding_registers(addr, msg.unit_id, &req, handler).await?.encode_to_bytes()
            }
            FunctionCode::WriteSingleCoil => {
                let req = WriteSingleCoilRequest::decode_from_bytes(&msg.body).map_err(|_| ModbusException::ServerDeviceFailure)?;
                Self::write_single_coil(addr, msg.unit_id, &req, handler).await?.encode_to_bytes()
            }
            FunctionCode::WriteSingleHoldingRegister => {
                let req = WriteSingleHoldingRegisterRequest::decode_from_bytes(&msg.body).map_err(|_| ModbusException::ServerDeviceFailure)?;
                Self::write_single_holding_register(addr, msg.unit_id, &req, handler)
                    .await?
                    .encode_to_bytes()
            }
            FunctionCode::WriteMultipleCoils => {
                let req = WriteMultipleCoilsRequest::decode_from_bytes(&msg.body).map_err(|_| ModbusException::ServerDeviceFailure)?;
                Self::write_multiple_coils(addr, msg.unit_id, &req, handler).await?.encode_to_bytes()
            }
            FunctionCode::WriteMultipleHoldingRegisters => {
                let req = WriteMultipleHoldingRegistersRequest::decode_from_bytes(&msg.body).map_err(|_| ModbusException::ServerDeviceFailure)?;
                Self::write_multiple_holding_registers(addr, msg.unit_id, &req, handler)
                    .await?
                    .encode_to_bytes()
            }
            FunctionCode::MaskWriteHoldingRegister => {
                let req = MaskWriteHoldingRegisterRequest::decode_from_bytes(&msg.body).map_err(|_| ModbusException::ServerDeviceFailure)?;
                Self::mask_write_holding_register(addr, msg.unit_id, &req, handler)
                    .await?
                    .encode_to_bytes()
            }
            FunctionCode::ModbusEncapsulatedInterface => {
                let req = ModbusEncapsulatedInterfaceRequest::decode_from_bytes(&msg.body).map_err(|_| ModbusException::ServerDeviceFailure)?;
                Self::modbus_encapsulated_interface(addr, msg.unit_id, &req, handler)
                    .await?
                    .encode_to_bytes()
            }
            _ => return Err(ModbusException::IllegalFunction),
        };

        Ok(bytes.map_err(|_| ModbusException::ServerDeviceFailure)?)
    }

    async fn read_coils<'a>(
        addr: SocketAddr,
        unit_id: u8,
        req: &ReadCoilsRequest,
        handler: &'a Arc<T>,
    ) -> Result<ReadCoilsResponse<'a>, ModbusException> {
        validate_input(req.address, req.length, READ_COILS_MAX_LEN)?;
        let values = handler.handle_read_coils(addr, unit_id, req.address, req.length).await?;
        validate_output(values.len(), req.length)?;
        Ok(ReadCoilsResponse { values })
    }

    async fn read_discrete_inputs<'a>(
        addr: SocketAddr,
        unit_id: u8,
        req: &ReadDiscreteInputsRequest,
        handler: &'a Arc<T>,
    ) -> Result<ReadDiscreteInputsResponse<'a>, ModbusException> {
        validate_input(req.address, req.length, READ_DISCRETE_INPUTS_MAX_LEN)?;
        let values = handler.handle_read_discrete_inputs(addr, unit_id, req.address, req.length).await?;
        validate_output(values.len(), req.length)?;
        Ok(ReadDiscreteInputsResponse { values })
    }

    async fn read_input_registers<'a>(
        addr: SocketAddr,
        unit_id: u8,
        req: &ReadInputRegistersRequest,
        handler: &'a Arc<T>,
    ) -> Result<ReadInputRegistersResponse<'a>, ModbusException> {
        validate_input(req.address, req.length, READ_INPUT_REGISTERS_MAX_LEN)?;
        let values = handler.handle_read_input_registers(addr, unit_id, req.address, req.length).await?;
        validate_output(values.len(), req.length)?;
        Ok(ReadInputRegistersResponse { values })
    }

    async fn read_holding_registers<'a>(
        addr: SocketAddr,
        unit_id: u8,
        req: &ReadHoldingRegistersRequest,
        handler: &'a Arc<T>,
    ) -> Result<ReadHoldingRegistersResponse<'a>, ModbusException> {
        validate_input(req.address, req.length, READ_HOLDING_REGISTERS_MAX_LEN)?;
        let values = handler.handle_read_holding_registers(addr, unit_id, req.address, req.length).await?;
        validate_output(values.len(), req.length)?;
        Ok(ReadHoldingRegistersResponse { values })
    }

    async fn write_single_coil(
        addr: SocketAddr,
        unit_id: u8,
        req: &WriteSingleCoilRequest,
        handler: &Arc<T>,
    ) -> Result<WriteSingleCoilResponse, ModbusException> {
        handler.handle_write_coils(addr, unit_id, req.address, &vec![req.value]).await?;
        Ok(WriteSingleCoilResponse {
            address: req.address,
            value: req.value,
        })
    }

    async fn write_single_holding_register(
        addr: SocketAddr,
        unit_id: u8,
        req: &WriteSingleHoldingRegisterRequest,
        handler: &Arc<T>,
    ) -> Result<WriteSingleHoldingRegisterResponse, ModbusException> {
        handler
            .handle_write_holding_registers(addr, unit_id, req.address, &vec![req.value])
            .await?;
        Ok(WriteSingleHoldingRegisterResponse {
            address: req.address,
            value: req.value,
        })
    }

    async fn write_multiple_coils<'a>(
        addr: SocketAddr,
        unit_id: u8,
        req: &WriteMultipleCoilsRequest<'a>,
        handler: &Arc<T>,
    ) -> Result<WriteMultipleCoilsResponse, ModbusException> {
        validate_input(req.address, req.values.len() as u16, WRITE_MULTIPLE_COILS_MAX_LEN)?;
        handler.handle_write_coils(addr, unit_id, req.address, &req.values).await?;
        Ok(WriteMultipleCoilsResponse {
            address: req.address,
            length: req.values.len() as u16,
        })
    }

    async fn write_multiple_holding_registers<'a>(
        addr: SocketAddr,
        unit_id: u8,
        req: &WriteMultipleHoldingRegistersRequest<'a>,
        handler: &Arc<T>,
    ) -> Result<WriteMultipleHoldingRegistersResponse, ModbusException> {
        validate_input(req.address, req.values.len() as u16, WRITE_MULTIPLE_HOLDING_REGISTERS_MAX_LEN)?;
        handler.handle_write_holding_registers(addr, unit_id, req.address, &req.values).await?;
        Ok(WriteMultipleHoldingRegistersResponse {
            address: req.address,
            length: req.values.len() as u16,
        })
    }

    async fn mask_write_holding_register(
        addr: SocketAddr,
        unit_id: u8,
        req: &MaskWriteHoldingRegisterRequest,
        handler: &Arc<T>,
    ) -> Result<MaskWriteHoldingRegisterResponse, ModbusException> {
        let current_value = handler.handle_read_holding_registers(addr, unit_id, req.address, 1).await?;
        validate_output(current_value.len(), 1)?;
        let current_value = current_value[0];
        let value = (current_value & req.and_mask) | (req.or_mask & (!req.and_mask));
        handler.handle_write_holding_registers(addr, unit_id, req.address, &vec![value]).await?;
        Ok(MaskWriteHoldingRegisterResponse {
            address: req.address,
            and_mask: req.and_mask,
            or_mask: req.or_mask,
        })
    }

    async fn modbus_encapsulated_interface<'a>(
        addr: SocketAddr,
        unit_id: u8,
        req: &ModbusEncapsulatedInterfaceRequest<'a>,
        handler: &'a Arc<T>,
    ) -> Result<ModbusEncapsulatedInterfaceResponse<'a>, ModbusException> {
        match req.kind {
            ModbusEncapsulatedInterfaceType::ReadDeviceIdentification => {
                let inner_req = ReadDeviceIdentificationRequest::decode_from_bytes(&req.data).map_err(|_| ModbusException::ServerDeviceFailure)?;
                let data = Self::read_device_identification(addr, unit_id, &inner_req, handler).await?;
                Ok(ModbusEncapsulatedInterfaceResponse {
                    kind: req.kind,
                    data: data.encode_to_bytes().map_err(|_| ModbusException::ServerDeviceFailure)?.into(),
                })
            }
            ModbusEncapsulatedInterfaceType::Unknown(kind) => {
                let data = handler.handle_modbus_encapsulated_interface(addr, unit_id, kind, &req.data).await?;
                Ok(ModbusEncapsulatedInterfaceResponse { kind: req.kind, data })
            }
        }
    }

    async fn read_device_identification<'a>(
        addr: SocketAddr,
        unit_id: u8,
        req: &ReadDeviceIdentificationRequest,
        handler: &'a Arc<T>,
    ) -> Result<ReadDeviceIdentificationResponse<'a>, ModbusException> {
        let device_info = handler.handle_read_device_identification(addr, unit_id).await?;

        let get_data = move |id: u8| -> Option<Vec<u8>> {
            match id {
                0 => Some(device_info.vendor_name.as_bytes().to_vec()),
                1 => Some(device_info.product_code.as_bytes().to_vec()),
                2 => Some(device_info.major_minor_revision.as_bytes().to_vec()),
                3 => Some(device_info.vendor_url.as_ref()?.as_bytes().to_vec()),
                4 => Some(device_info.product_name.as_ref()?.as_bytes().to_vec()),
                5 => Some(device_info.model_name.as_ref()?.as_bytes().to_vec()),
                6 => Some(device_info.user_application_name.as_ref()?.as_bytes().to_vec()),
                _ => Some(device_info.objects.get(&id)?.to_vec()),
            }
        };

        let data = get_data(req.object_id).ok_or(ModbusException::IllegalDataAddress)?;
        let max_object_id: u8;
        match req.device_id_code {
            ReadDeviceIdentificationIdCode::Unknown(_) => return Err(ModbusException::IllegalDataValue),
            ReadDeviceIdentificationIdCode::Individual => {
                return Ok(ReadDeviceIdentificationResponse {
                    device_id_code: req.device_id_code,
                    conformity_level: ReadDeviceIdentificationConformityLevel::ExtendedStreamAndIndividual,
                    more_follows: false,
                    next_object_id: 0,
                    objects: HashMap::from([(req.object_id, data.into())]),
                });
            }
            ReadDeviceIdentificationIdCode::Basic => max_object_id = 0x02,
            ReadDeviceIdentificationIdCode::Regular => max_object_id = 0x7F,
            ReadDeviceIdentificationIdCode::Extended => max_object_id = 0xFF,
        }

        let mut msg_length = 8 + 1 + 5 + 2 + data.len(); // 8 MSG, MEI = 1, RDI = 5, 2 per object
        let mut objects: HashMap<u8, Cow<[u8]>> = HashMap::from([(req.object_id, data.into())]);

        if msg_length > MSG_MAX_LENGTH {
            return Err(ModbusException::IllegalDataValue);
        }

        let mut next_object_id: u8 = 0;

        for id in (req.object_id + 1)..=max_object_id {
            match get_data(id) {
                None => continue,
                Some(data) => {
                    msg_length += 2 + data.len();
                    if msg_length > MSG_MAX_LENGTH {
                        next_object_id = id;
                        break;
                    }
                    objects.insert(id, data.into());
                }
            }
        }

        Ok(ReadDeviceIdentificationResponse {
            device_id_code: req.device_id_code,
            conformity_level: ReadDeviceIdentificationConformityLevel::ExtendedStreamAndIndividual,
            more_follows: next_object_id != 0,
            next_object_id,
            objects,
        })
    }
}

fn validate_input(address: u16, length: u16, max_length: u16) -> Result<(), ModbusException> {
    if length == 0 || length > max_length {
        return Err(ModbusException::IllegalDataValue);
    }
    u16::checked_add(address, length - 1).ok_or(ModbusException::IllegalDataAddress)?;
    Ok(())
}

fn validate_output(length: usize, expected_length: u16) -> Result<(), ModbusException> {
    if length != expected_length as usize {
        return Err(ModbusException::ServerDeviceFailure);
    }
    Ok(())
}
