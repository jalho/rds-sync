use std::{
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::{sleep, spawn},
    time::Duration,
};

use tungstenite::{accept, WebSocket};

fn handle(mut websocket: WebSocket<TcpStream>) {
    match websocket.write("foo".into()) {
        Ok(_) => {}
        Err(_) => todo!(),
    }

    match websocket.flush() {
        Ok(_) => {}
        Err(_) => todo!(),
    }

    sleep(Duration::from_millis(10000));
}

fn main() {
    let tcp_listener: TcpListener;

    /* TODO: Do I even need handles for connection handler threads? Maybe not
    because they need not be join()'d on because they're all spawned from one
    super thread that in turn is join()'d on. In other words, keeping parent
    thread alive should be enough to keep child threads alive. */
    let connection_handler_handles = Arc::new(Mutex::new(Vec::new()));

    match TcpListener::bind("127.0.0.1:8080") {
        Ok(n) => {
            tcp_listener = n;
        }
        Err(_) => todo!(),
    }

    let main_listener_handle = spawn(move || loop {
        let tcp_stream: TcpStream;
        let websocket: WebSocket<TcpStream>;

        match tcp_listener.accept() {
            Ok((n, _)) => {
                tcp_stream = n;
            }
            Err(_) => todo!(),
        }

        match accept(tcp_stream) {
            Ok(n) => {
                websocket = n;
            }
            Err(_) => todo!(),
        }

        match connection_handler_handles.lock() {
            Ok(mut n) => {
                let h = spawn(|| handle(websocket));
                n.push(h);
            }
            Err(_) => todo!(),
        }
    });

    match main_listener_handle.join() {
        Ok(_) => {}
        Err(_) => todo!(),
    }
}
