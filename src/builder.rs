use crate::retrying_tcp_stream::{RetryingTcpStream, TcpStreamSettings};
use crate::UniConnect;
use tokio_serial::{Serial, SerialPortSettings};

/// Helps create UniConnect with correct settings doesn't matter TCP or Serial port.
/// This `RetryingTcpOrSerial` will use RetryingTcpStream (this will reconnect internally on error) instead of
/// [TcpStream](tokio::net::TcpStream).
///
/// # Note
/// Use it more like example to create own builder (for your specific purpose) for UniConnect.
#[derive(Default)]
pub struct RetryingTcpOrSerial {
    connect_point: String,
    serial_port_settings: Option<SerialPortSettings>,
    tcp_settings: Option<TcpStreamSettings>,
}

impl RetryingTcpOrSerial {
    pub fn new(connect_point: String) -> Self {
        Self {
            connect_point,
            ..Default::default()
        }
    }

    /// This settings will be used if UniConnect will be using Serial under hood.
    pub fn set_serial_port_settings(&mut self, serial_port_settings: Option<SerialPortSettings>) {
        self.serial_port_settings = serial_port_settings;
    }

    /// This settings will be used if UniConnect will be using TCP under hood.
    pub fn set_tcp_settings(&mut self, tcp_settings: Option<TcpStreamSettings>) {
        self.tcp_settings = tcp_settings;
    }

    /// Consume builder and try create UniConnect.
    /// This build function will connect sync instead of async to make sure `connect_point` is
    /// correct
    pub fn build(self) -> Result<UniConnect, tokio::io::Error> {
        let res_socket_addr = self.connect_point.parse::<std::net::SocketAddr>();
        match res_socket_addr {
            Ok(socket_addr) => {
                let mut tcp_stream = RetryingTcpStream::connect(&socket_addr);
                if let Some(tcp_settings) = self.tcp_settings {
                    tcp_stream.set_tcp_settings(tcp_settings)?;
                }

                Ok(UniConnect::from(tcp_stream))
            }
            Err(_) => {
                let serial_settings = self.serial_port_settings.unwrap_or(Default::default());
                let serial = Serial::from_path(self.connect_point, &serial_settings)?;

                Ok(UniConnect::from(serial))
            }
        }
    }
}
