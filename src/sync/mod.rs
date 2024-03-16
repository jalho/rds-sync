use std::net::TcpStream;
use std::time::Duration;

use crate::rcon;
use crate::SystemTime;

/// Get game state over RCON.
pub fn sync_rcon(
    mut ws_rcon: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
    timeout_rcon: Duration,
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
        println!(
            "[{} {:?}] {} players, {} TCs",
            state.sync_time_ms,
            state.game_time,
            state.players.len(),
            state.tcs.len()
        );
    }
}
