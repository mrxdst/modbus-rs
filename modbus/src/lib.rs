mod client;
mod connection;
pub mod consts;
mod encoding;
mod function_code;
mod message;
mod messages;
mod modbus_encapsulated_interface;
mod modbus_exception;
mod server;

pub use client::{ModbusError, ModbusTCPClient};
pub use modbus_encapsulated_interface::DeviceIdentification;
pub use modbus_exception::ModbusException;
pub use server::{ModbusTCPServer, ModbusTCPServerHandler};
