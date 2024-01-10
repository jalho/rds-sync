use std::{net::TcpListener, thread::sleep, time::Duration};

use tungstenite::accept;

fn main() {
    match TcpListener::bind("127.0.0.1:8080") {
        Ok(tcp_listener) => match tcp_listener.accept() {
            Ok((tcp_stream, _)) => match accept(tcp_stream) {
                Ok(mut websocket) => match websocket.write("foo".into()) {
                    Ok(_) => match websocket.flush() {
                        Ok(_) => {
                            sleep(Duration::from_millis(2000));
                        }
                        Err(_) => todo!(),
                    },
                    Err(_) => todo!(),
                },
                Err(_) => todo!(),
            },
            Err(_) => todo!(),
        },
        Err(_) => todo!(),
    }
}
