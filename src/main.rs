use std::{
    net::{TcpListener, TcpStream},
    sync::{mpsc::channel, Arc, Mutex},
    thread::{sleep, spawn},
    time::{Duration, Instant},
};

use tungstenite::{accept, connect, stream::MaybeTlsStream, Message, WebSocket};

mod rcon;

fn handle(websocket: WebSocket<TcpStream>) {
    let (sender, receiver) = channel::<Message>();
    let ws_write = Arc::new(Mutex::new(websocket));
    let ws_read = Arc::clone(&ws_write);

    spawn(move || loop {
        match ws_read.try_lock() {
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

        /* Wait some time before acquiring the lock again so other work (e.g.
        sending RCON sync payloads) can continue using the WebSocket. */
        sleep(Duration::from_millis(1));
    });

    // TODO: construct RCON sync payload and send to the downstream
    let mut last = Instant::now();
    for i in 0..64 {
        match receiver.try_recv() {
            Ok(received) => {
                println!("Got RCON command from downstream: {}", received);
            }
            Err(_) => {}
        }

        match ws_write.try_lock() {
            Ok(mut ws) => {
                let now = Instant::now();
                match ws.write(
                    format!(
                        "RCON sync payload #{} -- elapsed: {:?}\n",
                        i,
                        last.elapsed()
                    )
                    .into(),
                ) {
                    Ok(_) => {}
                    Err(_) => {}
                }

                match ws.flush() {
                    Ok(_) => {}
                    Err(_) => {}
                }

                last = now;
            }
            Err(_) => {}
        }
        sleep(Duration::from_millis(1000));
    }
}

fn main() {
    let tcp_listener: TcpListener;

    /* Handle to RCON upstream WebSocket, intended for the remote-to-local RCON
    state synchronizing thread. */
    let rcon_upstream_sync: Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>;

    /* Handle to RCON upstream WebSocket, intended for the connected downstream
    clients handling threads' RCON command passing. */
    let rcon_upstream_cmd: Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>;

    let rcon_state_write = Arc::new(Mutex::new(rcon::State {
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

    let rcon_upstream_sync_handle = spawn(move || loop {
        match rcon_state_write.try_lock() {
            // lock acquired here
            Ok(state) => {
                // TODO: sync upstream with local state using rcon_upstream_sync
                println!("Syncing upstream RCON state!");
            }
            Err(_) => {}
        } // lock released here

        /* Wait some time before acquiring the lock again so other work can
        continue using the RCON state (i.e. reading it, sending updates to
        downstreams). */
        sleep(Duration::from_millis(1));
    });

    let main_listener_handle = spawn(move || loop {
        let tcp_stream: TcpStream;
        let websocket: WebSocket<TcpStream>;

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
                websocket = n;
                // TODO: use rcon_upstream_cmd to pass downstream sent commands to RCON upstream
                // TODO: use rcon_state_read to send RCON state to downstream clients
                spawn(|| handle(websocket));
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
