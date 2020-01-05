//! This module implement tokio::net::TcpStream in retryable way!

use std::convert::TryFrom;
use std::io::Read;
use std::io::Write;

use futures::try_ready;

use log::{debug, trace, warn};
use mio;
use tokio::io::{AsyncRead, AsyncWrite, Error};
use tokio::prelude::{Async, Future, Poll};

/// Holding settings state between reconnection
#[derive(Default, Clone)]
pub struct TcpStreamSettings {
    nodelay: bool,
}

// Handle connection state
enum ConnectionState {
    ConnectFuture(tokio::net::tcp::ConnectFuture),
    TcpStream(tokio::net::TcpStream),
}

pub struct RetryingTcpStream {
    addr: std::net::SocketAddr,
    settings: TcpStreamSettings,
    state: ConnectionState,
}

impl TryFrom<tokio::net::TcpStream> for RetryingTcpStream {
    type Error = Error;
    fn try_from(tcp_stream: tokio::net::TcpStream) -> Result<Self, Self::Error> {
        let settings = TcpStreamSettings {
            nodelay: tcp_stream.nodelay()?,
        };

        Ok(RetryingTcpStream {
            addr: tcp_stream.peer_addr()?,
            state: ConnectionState::TcpStream(tcp_stream),
            settings,
        })
    }
}

/// Implement creators
impl RetryingTcpStream {
    pub fn connect_with_settings(addr: &std::net::SocketAddr, settings: TcpStreamSettings) -> Self {
        Self {
            addr: addr.clone(),
            state: ConnectionState::ConnectFuture(tokio::net::TcpStream::connect(addr)),
            settings,
        }
    }

    pub fn connect(addr: &std::net::SocketAddr) -> Self {
        Self::connect_with_settings(addr, Default::default())
    }

    pub fn from_std(
        stream: std::net::TcpStream,
        handle: &tokio::reactor::Handle,
    ) -> Result<Self, Error> {
        let settings = TcpStreamSettings {
            nodelay: stream.nodelay()?,
        };

        Ok(Self {
            addr: stream.peer_addr()?,
            state: ConnectionState::TcpStream(tokio::net::TcpStream::from_std(stream, handle)?),
            settings,
        })
    }
}

/// Reimplement methods from TcpStream
impl RetryingTcpStream {
    pub fn poll_read_ready(&mut self, mask: mio::Ready) -> Result<Async<mio::Ready>, Error> {
        let ts = try_ready!(self.poll_into_tcp_stream());
        let res = ts.poll_read_ready(mask);
        self.call_reset_if_io_is_closed2(res)
    }

    pub fn poll_write_ready(&mut self) -> Result<Async<mio::Ready>, Error> {
        let ts = try_ready!(self.poll_into_tcp_stream());
        let res = ts.poll_write_ready();
        self.call_reset_if_io_is_closed2(res)
    }

    pub fn local_addr(&self) -> Result<std::net::SocketAddr, Error> {
        match &self.state {
            ConnectionState::ConnectFuture(_) => {
                Err(Error::from(tokio::io::ErrorKind::NotConnected))
            }
            ConnectionState::TcpStream(ts) => ts.local_addr(),
        }
    }

    pub fn peer_addr(&self) -> Result<std::net::SocketAddr, Error> {
        match &self.state {
            ConnectionState::ConnectFuture(_) => Ok(self.addr),
            ConnectionState::TcpStream(ts) => ts.peer_addr(),
        }
    }

    pub fn set_nodelay(&mut self, nodelay: bool) -> Result<(), Error> {
        match &self.state {
            ConnectionState::ConnectFuture(_) => {
                self.settings.nodelay = nodelay;
                Ok(())
            }
            ConnectionState::TcpStream(ts) => match ts.set_nodelay(nodelay) {
                Result::Ok(_) => {
                    self.settings.nodelay = nodelay;
                    Ok(())
                }
                Result::Err(err) => Err(err),
            },
        }
    }
}

impl RetryingTcpStream {
    pub fn set_tcp_settings(&mut self, tcp_settings: TcpStreamSettings) -> Result<(), Error> {
        self.set_nodelay(tcp_settings.nodelay)?;

        self.settings = tcp_settings;
        Ok(())
    }

    // Return NotReady until ConnectionState is diffrent than TcpStream
    fn poll_into_tcp_stream(&mut self) -> Poll<&mut tokio::net::TcpStream, Error> {
        match &mut self.state {
            ConnectionState::ConnectFuture(cf) => {
                let tcp_s = match cf.poll() {
                    Ok(Async::Ready(tcp_s)) => tcp_s,
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(err) => {
                        self.reset();
                        return Err(err);
                    }
                };
                self.state = ConnectionState::TcpStream(tcp_s);
                self.set_tcp_settings(self.settings.clone())?;
                debug!("RetryingTcpStream => change state ConnectFuture -> TcpStream")
            }
            ConnectionState::TcpStream(_) => (),
        };

        match self.state {
            ConnectionState::ConnectFuture(_) => unreachable!(),
            ConnectionState::TcpStream(ref mut ts) => Ok(Async::Ready(ts)),
        }
    }

    fn reset(&mut self) {
        warn!("RetryinTcpStream => reset was called!");
        self.state = ConnectionState::ConnectFuture(tokio::net::TcpStream::connect(&self.addr))
    }

    fn call_reset_if_io_is_closed2<T>(&mut self, res: Result<T, Error>) -> Result<T, Error> {
        use tokio::io::ErrorKind;
        match res {
            Ok(ok) => Ok(ok),
            Err(err) => {
                match err.kind() {
                    ErrorKind::WouldBlock => (),
                    _ => self.reset(),
                };
                Err(err)
            }
        }
    }
}

impl Read for RetryingTcpStream {
    /// # Note
    /// This is Async version of Read. It will panic outside of tash
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        trace!("RetryingTcpStream::read called");

        let ts = self.poll_into_tcp_stream()?;
        let r = match ts {
            Async::Ready(ts) => ts.read(buf),
            Async::NotReady => Err(std::io::ErrorKind::WouldBlock.into()),
        };

        self.call_reset_if_io_is_closed2(r)
    }
}

impl Write for RetryingTcpStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        trace!("RetryingTcpStream::write called");
        let ts = self.poll_into_tcp_stream()?;
        let r = match ts {
            Async::Ready(ts) => ts.write(buf),
            Async::NotReady => Err(std::io::ErrorKind::WouldBlock.into()),
        };

        self.call_reset_if_io_is_closed2(r)
    }

    fn flush(&mut self) -> Result<(), Error> {
        trace!("RetryingTcpStream::flush called");
        let ts = self.poll_into_tcp_stream()?;
        let r = match ts {
            Async::Ready(ts) => ts.flush(),
            Async::NotReady => Err(std::io::ErrorKind::WouldBlock.into()),
        };

        self.call_reset_if_io_is_closed2(r)
    }
}

// Logic is implemented inside Read and Write trait becouse we can't overwrite AsyncRead and
// AsyncWrite for Box<RetryingTcpStream>
// source: https://docs.rs/tokio-io/0.1.12/src/tokio_io/async_write.rs.html#149
impl AsyncRead for RetryingTcpStream {}

impl AsyncWrite for RetryingTcpStream {
    fn shutdown(&mut self) -> Poll<(), Error> {
        match &mut self.state {
            ConnectionState::ConnectFuture(_cf) => {
                // there is a chance when we call poll conection will resolve to TcpStream
                // we probably need add a Shutdowned state.
                unimplemented!();
            }
            ConnectionState::TcpStream(ts) => ts.shutdown(),
        }
    }
}
