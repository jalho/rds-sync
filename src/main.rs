use std::{net::TcpListener, time::Duration};

mod rcon;

fn main() {
    let _tcp_listener: TcpListener;
    let _rcon_command_timeout = Duration::from_millis(1000);

    let _state = rcon::State {
        players: vec![],
        tcs: vec![],
        game_time: rcon::EnvTime(0.0),
        sync_time_ms: 0,
    };

    match tungstenite::connect("ws://127.0.0.1:28016/Your_Rcon_Password") {
        Ok((_ws, _)) => {
            println!("Connected to RCON upstream WebSocket endpoint!");
        }
        Err(err_connect_rcon) => {
            eprintln!(
                "Failed to connect to RCON upstream WebSocket endpoint! {}",
                err_connect_rcon
            );
        }
    }
    println!("Dropped connection to RCON upstream WebSocket endpoint!");

    match TcpListener::bind("0.0.0.0:8080") {
        Ok(n) => {
            _tcp_listener = n;
        }
        Err(_) => todo!(),
    }
}
