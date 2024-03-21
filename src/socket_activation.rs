use listenfd::ListenFd;
use rocket::listener::{Bindable, Endpoint};
use rocket::tokio::net::TcpListener;
use std::io;

pub(crate) fn bindable_from_env() -> io::Result<Option<SocketActivation>> {
    ListenFd::from_env()
        .take_tcp_listener(0)
        .map(|r| r.map(SocketActivation))
}

#[derive(Debug)]
pub(crate) struct SocketActivation(std::net::TcpListener);

impl Bindable for SocketActivation {
    type Listener = TcpListener;
    type Error = io::Error;

    async fn bind(self) -> io::Result<Self::Listener> {
        self.0.set_nonblocking(true)?;
        TcpListener::from_std(self.0)
    }

    fn bind_endpoint(&self) -> io::Result<Endpoint> {
        Ok(Endpoint::new(self.0.local_addr()?))
    }
}
