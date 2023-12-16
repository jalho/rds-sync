use std::net::TcpStream;

use tungstenite::{stream::MaybeTlsStream, WebSocket};

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
    let mut websocket: WebSocket<MaybeTlsStream<TcpStream>>;
    match tungstenite::connect(&addr) {
        Ok((ws, _)) => {
            websocket = ws;
        }
        Err(_) => {
            // TODO: don't print RCON password to stdout
            eprintln!("Failed to connect a WebSocket to '{}'", &addr);
            std::process::exit(1);
        }
    };

    let timeout = std::time::Duration::from_millis(500);

    let playerlist_response_parsed = rcon::playerlist(&mut websocket, timeout).unwrap();
    for player_info in &playerlist_response_parsed {
        let playerlist_pretty = serde_json::to_string_pretty(&player_info).unwrap();
        println!("{}", playerlist_pretty);
    }

    rcon::time(&mut websocket, timeout);
}
