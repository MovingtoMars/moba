extern crate moba;
extern crate clap;

use std::net;
use clap::{Arg, App};

fn main() {
    let matches = App::new("moba")
        .version("alpha")
        .author("<definitelynotliam@gmail.com>")
        .arg(
            Arg::with_name("server")
                .short("s")
                .long("server")
                .value_name("SERVER")
                .help("Sets the server address to connect to")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("user")
                .short("u")
                .long("user")
                .value_name("USERNAME")
                .help("Sets the username to use")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("team")
                .short("t")
                .long("team")
                .value_name("TEAM_ID")
                .help("Sets the team to join")
                .takes_value(true),
        )
        .get_matches();

    println!("Alpha Client");

    let name = matches.value_of("user").unwrap();
    let server = matches.value_of("server").unwrap_or("127.0.0.1");
    let team = matches
        .value_of("team")
        .map(|t| moba::common::Team(t.parse().unwrap()));

    let addr = net::SocketAddrV4::new(server.parse().unwrap(), moba::common::DEFAULT_PORT);

    let mut client = moba::client::Client::new(name.to_owned(), team);

    match client.connect(addr) {
        Ok(()) => {}
        Err(e) => println!("Error connecting: {}", e),
    }
}
