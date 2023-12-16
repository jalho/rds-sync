use std::net::TcpStream;
use tungstenite::{stream::MaybeTlsStream, WebSocket};

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    name: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct RconResponse {
    Message: String,
    Identifier: u32,
    Type: String,
    Stacktrace: String,
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct RconCommand {
    Identifier: u32,
    Message: String,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct PlayerInfo {
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
type PlayerList = Vec<PlayerInfo>;

fn main() {
    let addr: &str = "ws://rds-remote:28016/SET_ME"; // TODO: get RCON password as input
    let (websocket, _) = tungstenite::connect(addr).unwrap();

    // TODO: use "playerlistpos" instead to get player list with positions -- not a JSON response!
    let playerlist_rcon_symbol = "playerlist";
    let timeout = std::time::Duration::from_millis(500);
    println!("RCON: {}", playerlist_rcon_symbol);
    let playerlist_response_raw = send_rcon_command(websocket, playerlist_rcon_symbol, timeout);
    let result_playerlist_response_parsed: Result<PlayerList, _> = serde_json::from_str(&playerlist_response_raw);
    let playerlist_response_parsed = result_playerlist_response_parsed.unwrap();
    for item in &playerlist_response_parsed {
        let playerlist_pretty = serde_json::to_string_pretty(&item).unwrap();
        println!("{}", playerlist_pretty);
    }
}

fn send_rcon_command(
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
        let elapsed = timestamp_send.elapsed().unwrap();
        if elapsed >= timeout {
            todo!(); // TODO: return some kinda error
        }

        let ws_message_in = socket.read().unwrap(); // TODO: if no message is ever received, we'll be stuck here. fix!
        let text = ws_message_in.to_text().unwrap();
        let rcon_response: RconResponse = serde_json::from_str(text).unwrap();
        if rcon_response.Identifier == rcon_command.Identifier {
            return rcon_response.Message;
        }
    }
}
