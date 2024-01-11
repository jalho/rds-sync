use std::{
    net::{TcpListener, TcpStream},
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

        spawn(|| handle(websocket));
    });

    match main_listener_handle.join() {
        Ok(_) => {}
        Err(_) => todo!(),
    }
}
