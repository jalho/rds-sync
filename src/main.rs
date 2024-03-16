use std::{
    net::{TcpListener, TcpStream},
    time::{Duration, SystemTime},
};

mod config;
mod rcon;
mod sync;

fn main() {
    // network resources
    let _tcp_listener: TcpListener;
    let ws_rcon: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

    // constants
    let timeout_rcon = Duration::from_millis(1000);
    let listen_addr: &str = "0.0.0.0:8080";

    // config
    let config: config::Config;
    config = config::Config::get();

    match tungstenite::connect(config.rcon_connection) {
        Ok((ws, _)) => {
            println!("Connected to RCON upstream WebSocket endpoint!");
            ws_rcon = ws;
            sync::sync_rcon(ws_rcon, timeout_rcon);
        }
        Err(err_connect_rcon) => {
            eprintln!(
                "Failed to connect to RCON upstream WebSocket endpoint! {}",
                err_connect_rcon
            );
        }
    }
    println!("Dropped connection to RCON upstream WebSocket endpoint!");

    match TcpListener::bind(listen_addr) {
        Ok(n) => {
            _tcp_listener = n;
        }
        Err(_) => todo!(),
    }
}
