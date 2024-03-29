use std::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

use tungstenite::{handshake::server::NoCallback, HandshakeError, ServerHandshake};
mod config;
mod rcon;
mod sync;

#[derive(Debug)]
/// All the fatal errors that shall make the program terminate.
enum ErrMainFatal {
    /// All sorts of errors coming from `tungstenite`.
    Tungstenite(tungstenite::Error),
    StdIo(std::io::Error),
    Handshake(HandshakeError<ServerHandshake<TcpStream, NoCallback>>),
}
impl From<tungstenite::Error> for ErrMainFatal {
    fn from(e: tungstenite::Error) -> Self {
        Self::Tungstenite(e)
    }
}
impl From<std::io::Error> for ErrMainFatal {
    fn from(e: std::io::Error) -> Self {
        Self::StdIo(e)
    }
}
impl From<HandshakeError<ServerHandshake<TcpStream, NoCallback>>> for ErrMainFatal {
    fn from(e: HandshakeError<ServerHandshake<TcpStream, NoCallback>>) -> Self {
        Self::Handshake(e)
    }
}

fn main() -> Result<(), ErrMainFatal> {
    // network resources
    let tcp_listener: TcpListener;

    // main threads
    let th_rcon_sync: thread::JoinHandle<()>;
    let th_ws_server: JoinHandle<Result<(), ErrMainFatal>>;
    let (tx, rx) = mpsc::channel::<rcon::State>();

    // constants
    let timeout_rcon = Duration::from_millis(1000);
    let listen_addr: &str = "0.0.0.0:8080";

    // config
    let config: config::Config;
    config = config::Config::get();

    let (ws_rcon, _) = tungstenite::connect(config.rcon_connection)?;
    println!("Connected to RCON upstream WebSocket endpoint!");
    th_rcon_sync = thread::spawn(move || sync::sync_rcon(ws_rcon, timeout_rcon, tx));

    tcp_listener = TcpListener::bind(listen_addr)?;
    println!("Listen address bound!");
    th_ws_server = thread::spawn(move || sync::accept_websockets(tcp_listener, &rx));

    let _ = th_rcon_sync.join();
    let _ = th_ws_server.join();
    return Ok(());
}
