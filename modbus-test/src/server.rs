use std::{borrow::Cow, collections::HashMap, error::Error, net::SocketAddr, sync::Arc};

use modbus::{DeviceIdentification, ModbusException, ModbusTCPServer, ModbusTCPServerHandler};
use tokio::{net::TcpListener, signal, sync::Mutex};

use super::args::Cli;

pub async fn run(args: Cli) -> Result<(), Box<dyn Error>> {
    let device_info = DeviceIdentification {
        vendor_name: env!("CARGO_PKG_NAME").into(),
        product_code: env!("CARGO_PKG_NAME").into(),
        major_minor_revision: env!("CARGO_PKG_VERSION").into(),
        model_name: None,
        product_name: Some(env!("CARGO_PKG_NAME").into()),
        user_application_name: None,
        vendor_url: None,
        objects: HashMap::new(),
    };

    let handler = Arc::new(ServerImpl {
        device_info,
        coils: Vec::from_iter((0..=0xFFFF).map(|v| v % 2 == 0)).into(),
        holding_registers: Vec::from_iter(0..=0xFFFF).into(),
    });

    let listener = TcpListener::bind(format!("localhost:{}", args.port)).await?;
    let listener_str = listener.local_addr()?.to_string();

    _ = ModbusTCPServer::run(listener, handler);

    println!("Server listening on {}. Press Ctrl-C to stop.", listener_str);

    signal::ctrl_c().await?;

    Ok(())
}

struct ServerImpl<'a> {
    device_info: DeviceIdentification<'a>,
    coils: Mutex<Vec<bool>>,
    holding_registers: Mutex<Vec<u16>>,
}

impl ModbusTCPServerHandler for ServerImpl<'static> {
    async fn accept_connection(&self, addr: SocketAddr) -> bool {
        println!("[{}] Connected", addr);
        true
    }

    async fn disconnected(&self, addr: SocketAddr) -> () {
        println!("[{}] Disconnected", addr);
    }

    async fn handle_read_coils(&self, addr: SocketAddr, unit_id: u8, address: u16, length: u16) -> Result<Cow<'_, [bool]>, ModbusException> {
        println!(
            "[{}] Read coils: unit: {}, address: 0{:05}-0{:05}",
            addr,
            unit_id,
            address,
            address + (length - 1)
        );
        let coils = self.coils.lock().await;
        let result = &coils[address as usize..address as usize + length as usize];
        Ok(result.to_vec().into())
    }

    async fn handle_read_discrete_inputs(&self, addr: SocketAddr, unit_id: u8, address: u16, length: u16) -> Result<Cow<'_, [bool]>, ModbusException> {
        println!(
            "[{}] Read discrete inputs: unit: {}, address: 1{:05}-1{:05}",
            addr,
            unit_id,
            address,
            address + (length - 1)
        );
        Ok((address..=(address + (length - 1))).map(|v| v % 2 == 0).collect())
    }

    async fn handle_read_input_registers(&self, addr: SocketAddr, unit_id: u8, address: u16, length: u16) -> Result<Cow<'_, [u16]>, ModbusException> {
        println!(
            "[{}] Read input registers: unit: {}, address: 3{:05}-3{:05}",
            addr,
            unit_id,
            address,
            address + (length - 1)
        );
        Ok((address..=(address + (length - 1))).collect())
    }

    async fn handle_read_holding_registers(&self, addr: SocketAddr, unit_id: u8, address: u16, length: u16) -> Result<Cow<'_, [u16]>, ModbusException> {
        println!(
            "[{}] Read holding registers: unit: {}, address: 4{:05}-4{:05}",
            addr,
            unit_id,
            address,
            address + (length - 1)
        );
        let holding_registers = self.holding_registers.lock().await;
        let result = &holding_registers[address as usize..address as usize + length as usize];
        Ok(result.to_vec().into())
    }

    async fn handle_write_coils(&self, addr: SocketAddr, unit_id: u8, address: u16, values: &[bool]) -> Result<(), ModbusException> {
        println!(
            "[{}] Write coils: unit: {}, address: 0{:05}-0{:05}, values: {:?}",
            addr,
            unit_id,
            address,
            address as usize + values.len() - 1,
            values
        );
        let mut coils = self.coils.lock().await;
        for i in 0..values.len() {
            coils[address as usize + i] = values[i];
        }
        Ok(())
    }

    async fn handle_write_holding_registers(&self, addr: SocketAddr, unit_id: u8, address: u16, values: &[u16]) -> Result<(), ModbusException> {
        println!(
            "[{}] Write holding registers: unit: {}, address: 4{:05}-4{:05}, values: {:?}",
            addr,
            unit_id,
            address,
            address as usize + values.len() - 1,
            values
        );
        let mut holding_registers = self.holding_registers.lock().await;
        for i in 0..values.len() {
            holding_registers[address as usize + i] = values[i];
        }
        Ok(())
    }

    async fn handle_read_device_identification(&self, addr: SocketAddr, unit_id: u8) -> Result<Cow<'_, DeviceIdentification<'_>>, ModbusException> {
        println!("[{}] Read device identification: unit: {}", addr, unit_id);
        Ok(Cow::Borrowed(&self.device_info))
    }
}
