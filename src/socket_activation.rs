use listenfd::ListenFd;
use rocket::listener::tcp::TcpListener;
use std::io;

pub(crate) fn listener_from_env() -> io::Result<Option<TcpListener>> {
    ListenFd::from_env()
        .take_tcp_listener(0)
        .and_then(|r| r.map(from_std).transpose())
}

fn from_std(listener: std::net::TcpListener) -> io::Result<TcpListener> {
    listener.set_nonblocking(true)?;
    TcpListener::from_std(listener)
}
