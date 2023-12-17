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

        // send to downstreams, prune dead connections
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

        for dead in &dead_downstreams {
            downstreams.remove(&dead);
        }
        // log updated (dead pruned) connected downstreams count
        if dead_downstreams.len() > 0 {
            println!("Connected downstream clients count: {}", downstreams.len());
        }

        std::thread::sleep(sync_interval);
    });

    let listener = std::net::TcpListener::bind("0.0.0.0:1234").unwrap();
    println!("{:?}", listener);
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        // TODO: add some kinda auth
        let websocket = tungstenite::accept(stream).unwrap();
        let mut downstreams = downstreams.lock().unwrap();
        downstreams.insert(Uuid::new_v4(), websocket);

        // log updated (new added) connected downstreams count
        println!("Connected downstream clients count: {}", downstreams.len());
    }

    // TODO: join spawned threads at interrupt?
    sync.join().unwrap();
}
