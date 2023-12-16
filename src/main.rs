mod rcon;

#[derive(clap::Parser, Debug)]
struct Args {
    #[arg(long)]
    rcon_password: String,
}

fn main() {
    // TODO: get fs path to some *.sh config file as arg and attempt to read RCON password from there
    let args = <Args as clap::Parser>::parse();
    let addr = format!("ws://rds-remote:28016/{}", args.rcon_password);
    let (websocket, _) = tungstenite::connect(addr).unwrap();

    let timeout = std::time::Duration::from_millis(500);
    let playerlist_response_parsed = rcon::playerlist(websocket, timeout).unwrap();
    for player_info in &playerlist_response_parsed {
        let playerlist_pretty = serde_json::to_string_pretty(&player_info).unwrap();
        println!("{}", playerlist_pretty);
    }
}
