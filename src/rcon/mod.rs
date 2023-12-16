use std::net::TcpStream;
use tungstenite::{stream::MaybeTlsStream, WebSocket};

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct RconCommand {
    Identifier: u32,
    Message: String,
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct RconResponse {
    Message: String,
    Identifier: u32,
    Type: String,
    Stacktrace: String,
}

pub fn send_rcon_command(
    mut socket: WebSocket<MaybeTlsStream<TcpStream>>,
    rcon_symbol: &str,
    timeout: std::time::Duration,
) -> String {
    let mut rng = rand::thread_rng();
    let command_id = rand::Rng::gen_range(&mut rng, 0..9999);
    let rcon_command = RconCommand {
        Identifier: command_id,
        Message: rcon_symbol.to_string(),
    };
    let cmd_serialized = serde_json::to_string(&rcon_command).unwrap();
    let ws_message_out = tungstenite::protocol::Message::text(cmd_serialized);

    let timestamp_send = std::time::SystemTime::now();
    socket.write(ws_message_out).unwrap();
    socket.flush().unwrap();

    loop {
        // only wait for a relevant response message till timeout
        let elapsed = timestamp_send.elapsed().unwrap();
        if elapsed >= timeout {
            todo!(); // TODO: return some kinda error
        }

        // TODO: if no message is ever received, we'll be stuck here. fix! (make the given timeout cover this case too)
        let ws_message_in = socket.read().unwrap();
        let text = ws_message_in.to_text().unwrap();
        let rcon_response: RconResponse = serde_json::from_str(text).unwrap();
        if rcon_response.Identifier == rcon_command.Identifier {
            return rcon_response.Message;
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PlayerInfo {
    Address: String,
    ConnectedSeconds: u32,
    CurrentLevel: f64,
    DisplayName: String,
    Health: f64,
    OwnerSteamID: String,
    Ping: u32,
    SteamID: String,
    UnspentXp: f64,
    VoiationLevel: f64,
}
pub type PlayerList = Vec<PlayerInfo>;

pub fn playerlist(
    websocket: WebSocket<MaybeTlsStream<TcpStream>>,
    timeout: std::time::Duration,
) -> Result<PlayerList, serde_json::Error> {
    let playerlist_rcon_symbol = "playerlist";
    let playerlist_response_raw = send_rcon_command(websocket, playerlist_rcon_symbol, timeout);
    let result_playerlist_response_parsed = serde_json::from_str(&playerlist_response_raw);
    return result_playerlist_response_parsed;
}
