use crate::rcon;
use crate::ErrMainFatal;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
use std::time::SystemTime;
use tungstenite::WebSocket;

/// Get game state over RCON.
pub fn sync_rcon(
    mut ws_rcon: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
    timeout_rcon: Duration,
    sender: Sender<rcon::State>,
) {
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

        match sender.send(state) {
            Ok(_) => {
                println!("Sent a state update!");
            }
            Err(err) => {
                eprintln!("Failed to send a state update! {}", err);
            }
        }
    }
}

pub fn accept_websockets(
    tcp_listener: TcpListener,
    receiver: &Receiver<rcon::State>,
) -> Result<(), ErrMainFatal> {
    loop {
        let (tcp_stream, _) = tcp_listener.accept()?;
        println!("TCP accepted!");
        let ws_downstream = tungstenite::accept(tcp_stream)?;
        println!("WebSocket accepted!");
        sync_downstream(ws_downstream, receiver);
    }
}

pub fn sync_downstream(ws_downstream: WebSocket<TcpStream>, receiver: &Receiver<rcon::State>) {
    loop {
        match receiver.recv() {
            Ok(_) => {
                println!("Got something!");
            }
            Err(err) => {
                eprintln!("Failed to get something!");
            }
        }
    }
    println!("Dropping downstream connection!");
}
