use std::{
    net::{Ipv4Addr, TcpListener, TcpStream},
    process::exit,
};

#[allow(unused)]
fn handle_connection(connection: TcpStream) {
    println!("Connection open {}", connection.peer_addr().unwrap());

    println!("Connection close {}", connection.peer_addr().unwrap());
}

#[allow(unused)]
fn main() {
    const PORT: u16 = 4221;

    let server = TcpListener::bind((Ipv4Addr::new(0, 0, 0, 0), PORT)).unwrap_or_else(|err| {
        eprintln!("Server: Listen {}", err.to_string());
        exit(1);
    });

    println!("Server listening at {}", server.local_addr().unwrap());

    for connection in server.incoming() {
        match connection {
            Ok(connection) => handle_connection(connection),
            Err(err) => {
                println!("Could not successfully accept {}", err.to_string())
            }
        }
    }
}
