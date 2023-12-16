mod rcon;

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

fn main() {
    let addr: &str = "ws://rds-remote:28016/SET_ME"; // TODO: get RCON password as input
    let (websocket, _) = tungstenite::connect(addr).unwrap();

    let timeout = std::time::Duration::from_millis(500);
    let playerlist_response_parsed = rcon::playerlist(websocket, timeout).unwrap();
    for player_info in &playerlist_response_parsed {
        let playerlist_pretty = serde_json::to_string_pretty(&player_info).unwrap();
        println!("{}", playerlist_pretty);
    }
}
