extern crate moba;

fn main() {
    println!("Alpha Server");

    let mut game = moba::server::Server::new();
    game.serve(moba::common::DEFAULT_PORT);
}
