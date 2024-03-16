use std::{
    net::{TcpListener, TcpStream},
    time::{Duration, SystemTime},
};

mod config;
mod rcon;

fn main() {
    // network resources
    let _tcp_listener: TcpListener;
    let mut ws_rcon: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

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
            loop {
                let game_time = rcon::env_time(&mut ws_rcon, &timeout_rcon);
                let playerlist = rcon::global_playerlist(&mut ws_rcon, &timeout_rcon);
                let playerlistpos = rcon::global_playerlistpos(&mut ws_rcon, &timeout_rcon);
                let players = rcon::merge_playerlists(playerlistpos, playerlist);
                let tcs = rcon::global_listtoolcupboards(&mut ws_rcon, &timeout_rcon);
                let sync_time_ms = SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let state = rcon::State {
                    players,
                    tcs,
                    game_time,
                    sync_time_ms,
                };
                println!(
                    "[{} {:?}] {} players, {} TCs",
                    state.sync_time_ms,
                    state.game_time,
                    state.players.len(),
                    state.tcs.len()
                );
            }
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
