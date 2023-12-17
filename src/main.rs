use tungstenite::{stream::MaybeTlsStream, WebSocket};

mod rcon;

#[derive(clap::Parser, Debug)]
struct Args {
    #[arg(long)]
    rcon_password: String,
}

fn get_current_time_utc() -> std::time::Duration {
    return std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
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
    let mut websocket = connect_rcon(&addr);

    let rcon_command_timeout = std::time::Duration::from_millis(1000);
    let sync_interval = std::time::Duration::from_millis(200);
    let state = std::sync::Arc::new(std::sync::Mutex::new(rcon::State {
        players: vec![],
        tcs: vec![],
        game_time: rcon::EnvTime(0.0),
        sync_time: std::time::Duration::ZERO,
    }));

    let arc_state_sync_upstream = state.clone();
    let sync = std::thread::spawn(move || loop {
        let mut state = arc_state_sync_upstream.lock().unwrap();

        state.sync_time = get_current_time_utc();

        // sync game time
        state.game_time = rcon::env_time(&mut websocket, &rcon_command_timeout);

        // sync players
        let playerlist = rcon::global_playerlist(&mut websocket, &rcon_command_timeout);
        let playerlistpos = rcon::global_playerlistpos(&mut websocket, &rcon_command_timeout);
        let players = rcon::merge_playerlists(playerlistpos, playerlist);
        state.players = players;

        // sync TCs
        state.tcs = rcon::global_listtoolcupboards(&mut websocket, &rcon_command_timeout);

        // TODO: Sync with downstreams

        std::thread::sleep(sync_interval);
    });

    // TODO: stay alive till interrupt
    std::thread::sleep(std::time::Duration::from_secs(60));
    sync.join().unwrap();

    // TODO: accept client WebSocket connections
    //       - check auth or just use firewall?

    // TODO: sync local RCON state with connected clients regularly
    //       - aggregate state (e.g. responses of rcon::global_playerlist and
    //         rcon::global_playerlistpos should be merged)
}
