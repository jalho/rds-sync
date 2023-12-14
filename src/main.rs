fn main() {
    let addr: &str = "ws://127.0.0.1:8080";
    let (mut websocket, _) = tungstenite::connect(addr).unwrap();
    let message = websocket.read().unwrap();
    let text = message.to_text().unwrap();
    println!("{}", text);
}
