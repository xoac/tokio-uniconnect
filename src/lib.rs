//! This crate create abstract layer over common connections types. At this moment it support:
//! * [tokio::net::TcpStream](tokio::net::TcpStream)
//! * [RetringTcpStream](retrying_tcp_stream::RetryingTcpStream) -- only via
//! [builder](builder::RetryingTcpOrSerial)
//! * [tokio_serial::Serial](tokio_serial::Serial)
//!
//! Idea is to create builder that will parse connection point and create proper UniConnect. An
//! example builder can be found in [builder](builder).
//!
//! Version 0.1 is compatible with tokio 0.1

/// Contains common builders for UniConnect
pub mod builder;

mod retrying_tcp_stream;

use crate::retrying_tcp_stream::RetryingTcpStream;
use tokio::net::TcpStream;
use tokio::prelude::{AsyncRead, AsyncWrite, Poll};
use tokio_serial::{self, Serial};

use std::io::{self, Read, Write};

use derive_more::From;

/// Universal Connector
#[derive(From)]
pub enum UniConnect {
    TcpStream(TcpStream),
    /// tokio TcpStream connector that reconnect on error
    RetringTcpStream(RetryingTcpStream),
    Serial(Serial),
}

impl Read for UniConnect {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            UniConnect::TcpStream(inner) => inner.read(buf),
            UniConnect::RetringTcpStream(inner) => inner.read(buf),
            UniConnect::Serial(inner) => inner.read(buf),
        }
    }
}

impl Write for UniConnect {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            UniConnect::TcpStream(inner) => inner.write(buf),
            UniConnect::RetringTcpStream(inner) => inner.write(buf),
            UniConnect::Serial(inner) => inner.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            UniConnect::TcpStream(inner) => inner.flush(),
            UniConnect::RetringTcpStream(inner) => inner.flush(),
            UniConnect::Serial(inner) => inner.flush(),
        }
    }
}

impl AsyncWrite for UniConnect {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        match self {
            UniConnect::TcpStream(inner) => inner.shutdown(),
            UniConnect::RetringTcpStream(inner) => inner.shutdown(),
            UniConnect::Serial(inner) => inner.shutdown(),
        }
    }

    fn poll_write(&mut self, buf: &[u8]) -> Poll<usize, io::Error> {
        match self {
            UniConnect::TcpStream(inner) => inner.poll_write(buf),
            UniConnect::RetringTcpStream(inner) => inner.poll_write(buf),
            UniConnect::Serial(inner) => inner.poll_write(buf),
        }
    }

    fn poll_flush(&mut self) -> Poll<(), io::Error> {
        match self {
            UniConnect::TcpStream(inner) => inner.poll_flush(),
            UniConnect::RetringTcpStream(inner) => inner.poll_flush(),
            UniConnect::Serial(inner) => inner.poll_flush(),
        }
    }
}

impl AsyncRead for UniConnect {
    fn poll_read(&mut self, buf: &mut [u8]) -> Poll<usize, io::Error> {
        match self {
            UniConnect::TcpStream(inner) => inner.poll_read(buf),
            UniConnect::RetringTcpStream(inner) => inner.poll_read(buf),
            UniConnect::Serial(inner) => inner.poll_read(buf),
        }
    }
}
