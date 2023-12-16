use std::net::TcpStream;
use tungstenite::{stream::MaybeTlsStream, WebSocket};

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct RconCommandIssued {
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
    socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    rcon_symbol: &str,
    timeout: std::time::Duration,
) -> String {
    let mut rng = rand::thread_rng();
    let command_id = rand::Rng::gen_range(&mut rng, 0..9999);
    let rcon_command = RconCommandIssued {
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

/// RCON command `global.playerlist`
pub fn global_playerlist(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    timeout: std::time::Duration,
) -> PlayerList {
    let rcon_symbol = "global.playerlist";
    let response_raw = send_rcon_command(websocket, rcon_symbol, timeout);
    let response_parsed = serde_json::from_str(&response_raw);
    return response_parsed.unwrap();
}

#[derive(Debug)]
pub struct EnvTime(f64);

/// RCON command `env.time`
pub fn env_time(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    timeout: std::time::Duration,
) -> EnvTime {
    let rcon_symbol = "env.time";
    let response_raw = send_rcon_command(websocket, rcon_symbol, timeout);

    // Match the float in e.g. `env.time: "10.63853"`
    let re = regex::Regex::new(r#"env\.time:\s*"(\d+\.\d+)""#).unwrap();
    let captures = re.captures(&response_raw).unwrap();
    let match_group = &captures[1];
    let float = match_group.parse::<f64>().unwrap();
    return EnvTime(float);
}
