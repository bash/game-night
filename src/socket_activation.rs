use listenfd::ListenFd;
use rocket::listener::tcp::TcpListener;
use std::io;

pub(crate) fn listener_from_env() -> io::Result<Option<TcpListener>> {
    ListenFd::from_env()
        .take_tcp_listener(0)
        .and_then(|r| r.map(TcpListener::from_std).transpose())
}
