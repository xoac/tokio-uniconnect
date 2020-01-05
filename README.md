# tokio-uniconnect

This crate create abstract layer over common connections types. At this moment it support:
* [tokio::net::TcpStream](tokio::net::TcpStream)
* [RetringTcpStream](retrying_tcp_stream::RetryingTcpStream) -- only via
[builder](builder::RetryingTcpOrSerial)
* [tokio_serial::Serial](tokio_serial::Serial)

Idea is to create builder that will parse connection point and create proper UniConnect. An
example builder can be found in [builder](builder).

Version 0.1 is compatible with tokio 0.1

License: MIT
