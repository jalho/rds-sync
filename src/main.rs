use std::net::TcpStream;
use tungstenite::{stream::MaybeTlsStream, WebSocket};

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct RconResponse {
    Message: String,
    Identifier: u8,
    Type: String,
    Stacktrace: String,
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct RconCommand {
    Identifier: u8,
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
    rcon_command: &str,
) -> String {
    let cmd = RconCommand {
        Identifier: 1, // TODO: construct an RCON payload with a dynamic "Identifier"
        Message: rcon_command.to_string()
    };
    let cmd_serialized = serde_json::to_string(&cmd).unwrap();
    let ws_message_out = tungstenite::protocol::Message::text(cmd_serialized);
    socket.write(ws_message_out).unwrap();
    socket.flush().unwrap();

    // TODO: read messages till some timeout until a message with matching "Identifier is received"

    let ws_message_in = socket.read().unwrap();
    let text = ws_message_in.to_text().unwrap();
    let p: RconResponse = serde_json::from_str(text).unwrap();
    return p.Message;
}
