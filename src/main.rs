use std::{
    net::{TcpListener, TcpStream},
    sync::{mpsc::channel, Arc, Mutex},
    thread::{sleep, spawn},
    time::{Duration, SystemTime},
};

use rcon::send_rcon_command;
use tungstenite::{accept, connect, stream::MaybeTlsStream, Message, WebSocket};

mod rcon;

fn handle(
    downstream: WebSocket<TcpStream>,
    upstream_write: Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>,
    state_read: Arc<Mutex<rcon::State>>,
    rcon_command_timeout: Duration,
) {
    let (sender, receiver) = channel::<Message>();
    let downstream_write = Arc::new(Mutex::new(downstream));
    let downstream_read = Arc::clone(&downstream_write);

    // get RCON commands from the downstream
    spawn(move || loop {
        match downstream_read.try_lock() {
            // lock acquired here
            Ok(mut ws) => match ws.read() {
                Ok(msg) => match sender.send(msg) {
                    Ok(_) => {}
                    Err(_) => todo!(),
                },
                Err(_) => {}
            },
            Err(_) => {}
        } // lock released here

        /* Sleep a while before acquiring the downstream socket lock again to
        allow another mechanism to use the socket. The other mechanism is the
        one that sends RCON state updates to the downstream. */
        sleep(Duration::from_millis(200));
    });

    // read incoming RCON commands from the downstream and send state updates to the downstream
    loop {
        match receiver.try_recv() {
            Ok(received) => match received {
                Message::Text(message) => match upstream_write.lock() {
                    Ok(mut sock) => {
                        println!("Passing RCON command to the upstream: '{}'", message);
                        send_rcon_command(&mut sock, &message, &rcon_command_timeout);
                    }
                    Err(_) => todo!(),
                },
                Message::Binary(_) => todo!(),
                Message::Ping(_) => todo!(),
                Message::Pong(_) => todo!(),
                Message::Close(_) => todo!(),
                Message::Frame(_) => todo!(),
            },
            Err(_) => {}
        }
        match state_read.lock() {
            // state read lock acquired here
            Ok(local_state_to_send) => {
                match downstream_write.lock() {
                    // downstream socket lock acquired here
                    Ok(mut ws) => {
                        let state_serialized: String;
                        match serde_json::to_string(&*local_state_to_send) {
                            Ok(serialized) => {
                                state_serialized = serialized;
                            }
                            Err(_) => todo!(),
                        }

                        match ws.write(state_serialized.into()) {
                            Ok(_) => {}
                            Err(_) => {}
                        }

                        match ws.flush() {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                    Err(_) => {}
                }
            } // downstream socket lock released here
            Err(_) => {}
        } // state read lock released here

        /* Sleep a while before acquiring the locks for upstream and downstream
        sockets and local state again to allow another mechanism to use them.
        The other mechanism is the one syncing upstream RCON state with local
        state. */
        sleep(Duration::from_millis(200));
    }
}

fn get_current_time_utc() -> u128 {
    return SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
}

fn main() {
    let tcp_listener: TcpListener;

    /* Handle to RCON upstream WebSocket, intended for the remote-to-local RCON
    state synchronizing thread. */
    let rcon_upstream_sync: Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>;

    /* Handle to RCON upstream WebSocket, intended for the connected downstream
    clients handling threads' RCON command passing. */
    let rcon_upstream_cmd: Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>;

    let rcon_command_timeout = Duration::from_millis(1000);

    let rcon_state_write: Arc<Mutex<rcon::State>>;
    rcon_state_write = Arc::new(Mutex::new(rcon::State {
        players: vec![],
        tcs: vec![],
        game_time: rcon::EnvTime(0.0),
        sync_time_ms: 0,
    }));
    let rcon_state_read = Arc::clone(&rcon_state_write);

    match connect("ws://192.168.0.104:28016/Your_Rcon_Password") {
        Ok((ws, _)) => {
            rcon_upstream_sync = Arc::new(Mutex::new(ws));
            rcon_upstream_cmd = Arc::clone(&rcon_upstream_sync);
        }
        Err(_) => todo!(),
    }

    match TcpListener::bind("0.0.0.0:8080") {
        Ok(n) => {
            tcp_listener = n;
        }
        Err(_) => todo!(),
    }

    // get RCON state from the upstream and update local state
    let rcon_upstream_sync_handle = spawn(move || loop {
        match rcon_state_write.lock() {
            // state write lock acquired here
            Ok(mut local_state_to_sync) => {
                match rcon_upstream_sync.lock() {
                    // upstream socket lock acquired here
                    Ok(mut socket) => {
                        local_state_to_sync.sync_time_ms = get_current_time_utc();

                        // sync game time
                        local_state_to_sync.game_time =
                            rcon::env_time(&mut socket, &rcon_command_timeout);

                        // sync players
                        let playerlist =
                            rcon::global_playerlist(&mut socket, &rcon_command_timeout);
                        let playerlistpos =
                            rcon::global_playerlistpos(&mut socket, &rcon_command_timeout);
                        let players = rcon::merge_playerlists(playerlistpos, playerlist);
                        local_state_to_sync.players = players;

                        // sync TCs
                        local_state_to_sync.tcs =
                            rcon::global_listtoolcupboards(&mut socket, &rcon_command_timeout);
                    }
                    Err(_) => todo!(),
                } // upstream socket lock released here
            }
            Err(_) => {}
        } // state write lock released here

        /* Sleep a while before acquiring the upstream sync socket and local
        state locks again to allow other mechanisms to use them. The other
        mechanisms are those passing on RCON commands from downstream to
        upstream and sending state updates to downstreams. */
        sleep(Duration::from_millis(200));
    });

    let main_listener_handle = spawn(move || loop {
        let tcp_stream: TcpStream;
        let downstream: WebSocket<TcpStream>;

        match tcp_listener.accept() {
            Ok((n, _)) => {
                /* Set non-blocking so that we can use the established WebSocket
                in a non-blocking way, i.e. periodically check for receivable
                messages from it while also periodically sending messages to it. */
                match n.set_nonblocking(true) {
                    Ok(_) => {}
                    Err(_) => todo!(),
                }
                tcp_stream = n;
            }
            Err(_) => todo!(),
        }

        /*  Wait for a WebSocket handshake because the underlying TCP stream is
        set non-blocking. The full handshake buffer is not immediately received
        and tungstenite::accept does not wait for it.

            TODO: Wait until the actual handshake is received, instead of an
                  arbitrary time that is hopefully sufficient! */
        sleep(Duration::from_micros(1000));
        match accept(tcp_stream) {
            Ok(n) => {
                downstream = n;
                let upstream_write = Arc::clone(&rcon_upstream_cmd);
                let state_read = Arc::clone(&rcon_state_read);
                spawn(move || handle(downstream, upstream_write, state_read, rcon_command_timeout));
            }
            /* Error here occurs e.g. when the handshake was only partially
            received. */
            Err(err) => {
                eprintln!(
                    "Error while attempting to accept a WebSocket handshake: {:?}",
                    err
                );
            }
        }
    });

    match main_listener_handle.join() {
        Ok(_) => {}
        Err(_) => todo!(),
    }
    match rcon_upstream_sync_handle.join() {
        Ok(_) => {}
        Err(_) => todo!(),
    }
}
