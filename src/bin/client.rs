extern crate moba;

use std::net;

fn main() {
    println!("Alpha Client");

    let name = std::env::args().nth(1).unwrap_or("Unnamed".into());

    let addr = net::SocketAddrV4::new(net::Ipv4Addr::new(127, 0, 0, 1), moba::common::DEFAULT_PORT);

    let mut client = moba::client::Client::new(name);

    match client.connect(addr) {
        Ok(()) => {}
        Err(e) => println!("Error connecting: {}", e),
    }
}
