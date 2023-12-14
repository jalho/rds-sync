fn main() {
    let addr: &str = "ws://127.0.0.1:8080";
    let (mut websocket, _) = tungstenite::connect(addr).unwrap();

    let playerlist = "{\"Identifier\":1,\"Message\":\"playerlist\"}";
    let message_out = tungstenite::protocol::Message::text(playerlist);
    websocket.write(message_out).unwrap();
    websocket.flush().unwrap();

    let message_in = websocket.read().unwrap();
    let text = message_in.to_text().unwrap();
    println!("{}", text);
}
