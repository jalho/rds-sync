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

    let rcon_command_timeout = std::time::Duration::from_millis(500);

    let players = rcon::global_playerlist(&mut websocket, rcon_command_timeout);
    for player in players {
        let playerlist_pretty = serde_json::to_string_pretty(&player).unwrap();
        println!("{}", playerlist_pretty);
    }

    let env_time = rcon::env_time(&mut websocket, rcon_command_timeout);
    println!("{:?}", env_time);
}
