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

struct RconSyncApi {
    command_timeout: Duration,
    state: Arc<Mutex<rcon::State>>,
    subscribed_clients: Arc<Mutex<HashMap<Uuid, WebSocket<TcpStream>>>>,
    sync_interval: Duration,
    upstream_socket: WebSocket<MaybeTlsStream<TcpStream>>,
}

/// Sync remote RCON upstream state with local state with a regular interval.
fn sync_periodic(mut api: RconSyncApi) {
    loop {
        let mut state = api.state.lock().unwrap();

        state.sync_time_ms = get_current_time_utc();

        // sync game time
        state.game_time = rcon::env_time(&mut api.upstream_socket, &api.command_timeout);

        // sync players
        let playerlist = rcon::global_playerlist(&mut api.upstream_socket, &api.command_timeout);
        let playerlistpos =
            rcon::global_playerlistpos(&mut api.upstream_socket, &api.command_timeout);
        let players = rcon::merge_playerlists(playerlistpos, playerlist);
        state.players = players;

        // sync TCs
        state.tcs = rcon::global_listtoolcupboards(&mut api.upstream_socket, &api.command_timeout);

        // send to downstreams
        let mut subscribed_clients = api.subscribed_clients.lock().unwrap();
        let mut dead_clients: Vec<Uuid> = vec![];
        for (id, socket) in subscribed_clients.iter_mut() {
            let state_serialized = serde_json::to_string(&*state).unwrap();
            match socket.send(state_serialized.into()) {
                Ok(_) => {}
                Err(_) => {
                    dead_clients.push(*id);
                }
            }
        }

        // prune dead connections
        for dead in &dead_clients {
            subscribed_clients.remove(&dead);
        }
        if dead_clients.len() > 0 {
            println!(
                "Connected downstream clients count: {}",
                subscribed_clients.len()
            );
        }

        std::thread::sleep(api.sync_interval);
    }
}

/// Accept downstream WebSocket connections.
fn accept_downstreams(connected_clients: Arc<Mutex<HashMap<Uuid, WebSocket<TcpStream>>>>) {
    let listener = std::net::TcpListener::bind("0.0.0.0:1234").unwrap();
    println!("{:?}", listener);
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        // TODO: add some kinda auth
        let websocket = tungstenite::accept(stream).unwrap();
        let mut downstreams = connected_clients.lock().unwrap();
        downstreams.insert(Uuid::new_v4(), websocket);

        // log updated (new added) connected downstreams count
        println!("Connected downstream clients count: {}", downstreams.len());
    }
}

fn main() {
    // TODO: get fs path to some *.sh config file as arg and attempt to read RCON password from there
    let args = <Args as clap::Parser>::parse();
    let addr = format!("ws://rds-remote:28016/{}", args.rcon_password);
    let upstream_socket = connect_rcon(&addr);

    let connected_clients: Arc<Mutex<HashMap<Uuid, WebSocket<TcpStream>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let subscribed_clients = connected_clients.clone();

    let command_timeout = Duration::from_millis(1000);
    let sync_interval = Duration::from_millis(200);
    let state = Arc::new(Mutex::new(rcon::State {
        players: vec![],
        tcs: vec![],
        game_time: rcon::EnvTime(0.0),
        sync_time_ms: 0,
    }));
    let state = state.clone();
    let api = RconSyncApi {
        command_timeout,
        state,
        subscribed_clients,
        sync_interval,
        upstream_socket,
    };

    let syncer = std::thread::spawn(|| sync_periodic(api));
    let acceptor = std::thread::spawn(|| accept_downstreams(connected_clients));

    // TODO: join spawned threads at interrupt?
    syncer.join().unwrap();
    acceptor.join().unwrap();
}
