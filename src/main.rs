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

fn main() {
    let addr: &str = "ws://37.27.15.73:28016/SET_ME"; // TODO: get as input
    let (websocket, _) = tungstenite::connect(addr).unwrap();

    let playerlist_response = send_rcon_command(websocket, "playerlist");
    println!("{:?}", playerlist_response);
}

fn send_rcon_command(
    mut socket: WebSocket<MaybeTlsStream<TcpStream>>,
    rcon_symbol: &str,
) -> String {
    let mut rng = rand::thread_rng();
    let command_id = rand::Rng::gen_range(&mut rng, 0..9999);
    let rcon_command = RconCommand {
        Identifier: command_id,
        Message: rcon_symbol.to_string(),
    };
    let cmd_serialized = serde_json::to_string(&rcon_command).unwrap();
    let ws_message_out = tungstenite::protocol::Message::text(cmd_serialized);
    socket.write(ws_message_out).unwrap();
    socket.flush().unwrap();

    // TODO: read messages till some timeout until a message with matching "Identifier is received"

    let ws_message_in = socket.read().unwrap();
    let text = ws_message_in.to_text().unwrap();
    let rcon_response: RconResponse = serde_json::from_str(text).unwrap();
    return rcon_response.Message;
}
