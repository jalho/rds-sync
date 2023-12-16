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
        let list_pretty = serde_json::to_string_pretty(&player).unwrap();
        println!("{}", list_pretty);
    }

    let env_time = rcon::env_time(&mut websocket, rcon_command_timeout);
    println!("{:?}", env_time);

    let players = rcon::global_playerlistpos(&mut websocket, rcon_command_timeout);
    for player in players {
        let list_pretty = serde_json::to_string_pretty(&player).unwrap();
        println!("{}", list_pretty);
    }

    let tcs = rcon::global_listtoolcupboards(&mut websocket, rcon_command_timeout);
    for tc in tcs {
        let list_pretty = serde_json::to_string_pretty(&tc).unwrap();
        println!("{}", list_pretty);
    }

    // TODO: add rcon::global_listtoolcupboards

    // TODO: sync remote RCON state with local state regularly
    //       - rcon::env_time
    //       - rcon::global_listtoolcupboards
    //       - rcon::global_playerlist
    //       - rcon::global_playerlistpos

    // TODO: accept client WebSocket connections
    //       - check auth or just use firewall?

    // TODO: sync local RCON state with connected clients regularly
    //       - aggregate state (e.g. responses of rcon::global_playerlist and
    //         rcon::global_playerlistpos should be merged)
}
