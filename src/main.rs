use std::{
    net::{TcpListener, TcpStream},
    thread::sleep,
    time::Duration,
};

use tungstenite::{accept, WebSocket};

fn main() {
    let tcp_listener: TcpListener;
    let tcp_stream: TcpStream;
    let mut websocket: WebSocket<TcpStream>;

    match TcpListener::bind("127.0.0.1:8080") {
        Ok(n) => {
            tcp_listener = n;
        }
        Err(_) => todo!(),
    }

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

    match websocket.write("foo".into()) {
        Ok(_) => {}
        Err(_) => todo!(),
    }

    match websocket.flush() {
        Ok(_) => {}
        Err(_) => todo!(),
    }

    sleep(Duration::from_millis(2000));
}
