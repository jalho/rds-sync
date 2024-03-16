use std::{net::TcpListener, thread, time::Duration};
mod config;
mod rcon;
mod sync;

fn main() -> Result<(), tungstenite::Error> {
    // network resources
    let _tcp_listener: TcpListener;

    // constants
    let timeout_rcon = Duration::from_millis(1000);
    // let listen_addr: &str = "0.0.0.0:8080";

    // config
    let config: config::Config;
    config = config::Config::get();

    let (ws_rcon, _) = tungstenite::connect(config.rcon_connection)?;
    println!("Connected to RCON upstream WebSocket endpoint!");

    let th_rcon_sync: thread::JoinHandle<()>;
    th_rcon_sync = thread::spawn(move || sync::sync_rcon(ws_rcon, timeout_rcon));

    // match TcpListener::bind(listen_addr) {
    //     Ok(n) => {
    //         _tcp_listener = n;
    //     }
    //     Err(_) => todo!(),
    // }

    let _ = th_rcon_sync.join();
    return Ok(());
}
