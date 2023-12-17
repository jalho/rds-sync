use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tungstenite::{stream::MaybeTlsStream, WebSocket};
use uuid::Uuid;

mod rcon;

#[derive(clap::Parser, Debug)]
struct Args {
    #[arg(long)]
    rcon_password: String,
}

fn get_current_time_utc() -> u128 {
    return SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
}

fn connect_rcon(addr: &String) -> WebSocket<MaybeTlsStream<std::net::TcpStream>> {
    let websocket: WebSocket<MaybeTlsStream<std::net::TcpStream>>;
    match tungstenite::connect(addr) {
        Ok((ws, _)) => {
            websocket = ws;
        }
        Err(_) => {
            // TODO: don't print RCON password to stdout
            eprintln!("Failed to connect a WebSocket to '{}'", &addr);
            std::process::exit(1);
        }
    };
    return websocket;
}

fn main() {
    // TODO: get fs path to some *.sh config file as arg and attempt to read RCON password from there
    let args = <Args as clap::Parser>::parse();
    let addr = format!("ws://rds-remote:28016/{}", args.rcon_password);
    let mut websocket_rcon_upstream = connect_rcon(&addr);

    let downstreams: Arc<Mutex<HashMap<Uuid, WebSocket<TcpStream>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let downstreams_clone = downstreams.clone();

    let listener = std::net::TcpListener::bind("0.0.0.0:8080").unwrap();
    let listener_handle = std::thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let websocket = tungstenite::accept(stream).unwrap();
            let mut downstreams = downstreams.lock().unwrap();
            downstreams.insert(Uuid::new_v4(), websocket);
        }
    });

    let rcon_command_timeout = Duration::from_millis(1000);
    let sync_interval = Duration::from_millis(200);
    let state = Arc::new(Mutex::new(rcon::State {
        players: vec![],
        tcs: vec![],
        game_time: rcon::EnvTime(0.0),
        sync_time_ms: 0,
    }));

    let arc_state_sync_upstream = state.clone();
    let sync = std::thread::spawn(move || loop {
        let mut state = arc_state_sync_upstream.lock().unwrap();

        state.sync_time_ms = get_current_time_utc();

        // sync game time
        state.game_time = rcon::env_time(&mut websocket_rcon_upstream, &rcon_command_timeout);

        // sync players
        let playerlist =
            rcon::global_playerlist(&mut websocket_rcon_upstream, &rcon_command_timeout);
        let playerlistpos =
            rcon::global_playerlistpos(&mut websocket_rcon_upstream, &rcon_command_timeout);
        let players = rcon::merge_playerlists(playerlistpos, playerlist);
        state.players = players;

        // sync TCs
        state.tcs =
            rcon::global_listtoolcupboards(&mut websocket_rcon_upstream, &rcon_command_timeout);

        let mut downstreams = downstreams_clone.lock().unwrap();
        let mut dead_downstreams: Vec<Uuid> = vec![];
        for (id, socket) in downstreams.iter_mut() {
            let serialized = serde_json::to_string(&*state).unwrap();
            match socket.send(serialized.into()) {
                Ok(_) => {}
                Err(_) => {
                    dead_downstreams.push(*id);
                }
            }
        }
        for dead in dead_downstreams {
            downstreams.remove(&dead);
        }

        println!(
            "Sync time {:?}, connected downstreams: {:?}",
            state.sync_time_ms,
            downstreams.len()
        );

        std::thread::sleep(sync_interval);
    });

    // TODO: stay alive till interrupt
    std::thread::sleep(Duration::from_secs(60));
    sync.join().unwrap();
    listener_handle.join().unwrap();

    // TODO: accept client WebSocket connections
    //       - check auth or just use firewall?

    // TODO: sync local RCON state with connected clients regularly
    //       - aggregate state (e.g. responses of rcon::global_playerlist and
    //         rcon::global_playerlistpos should be merged)
}
