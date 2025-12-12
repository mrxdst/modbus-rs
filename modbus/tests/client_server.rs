use std::{borrow::Cow, collections::HashMap, net::SocketAddr, sync::Arc};

use modbus::{DeviceIdentification, ModbusException, ModbusTCPClient, ModbusTCPServer, ModbusTCPServerHandler};
use tokio::net::{TcpListener, TcpSocket};

#[tokio::test]
pub async fn client_server() {
    let device_info = DeviceIdentification {
        vendor_name: "Test".into(),
        product_code: "Test".into(),
        major_minor_revision: "Test".into(),
        model_name: None,
        product_name: None,
        user_application_name: None,
        vendor_url: None,
        objects: HashMap::new(),
    };

    let handler = Arc::new(ServerImpl {
        device_info: device_info.clone(),
    });
    let listener = TcpListener::bind("[::1]:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    _ = ModbusTCPServer::run(listener, handler);

    let socket = TcpSocket::new_v6().unwrap();
    let stream = socket.connect(format!("[::1]:{port}").parse().unwrap()).await.unwrap();
    let (client, _) = ModbusTCPClient::new(stream);

    let read_device_info = client.read_device_identification(0).await.unwrap();

    assert_eq!(device_info, read_device_info);
}

struct ServerImpl<'a> {
    device_info: DeviceIdentification<'a>,
}

impl ModbusTCPServerHandler for ServerImpl<'static> {
    async fn handle_read_device_identification(&self, _addr: SocketAddr, _unit_id: u8) -> Result<Cow<'_, DeviceIdentification<'_>>, ModbusException> {
        Ok(Cow::Borrowed(&self.device_info))
    }
}
