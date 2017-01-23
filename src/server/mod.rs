use std::net::{self, TcpListener, TcpStream};
use std::io;
use std::thread;
use std::time;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use ecs;

use common::{self, Message, Stream, Game, Command, Hero, Point, EntityID};

const TICKS_PER_SECOND: u32 = 200;

pub struct Server {
    game: Game,
    streams: HashMap<EntityID, Stream>,
    joining_players: Arc<Mutex<Vec<(Stream, String)>>>,
}

impl Server {
    pub fn new() -> Self {
        Server {
            game: Game::new(),
            joining_players: Arc::new(Mutex::new(Vec::new())),
            streams: HashMap::new(),
        }
    }

    pub fn serve(&mut self, port: u16) {
        let jp = self.joining_players.clone();
        thread::spawn(move || {
            let addr = net::SocketAddrV4::new(net::Ipv4Addr::new(127, 0, 0, 1), port);

            let listener = TcpListener::bind(addr).unwrap();

            // accept connections and process them, spawning a new thread for each one
            for stream in listener.incoming() {
                let jp = jp.clone();
                thread::spawn(move || handle_client(stream.unwrap(), jp).unwrap());
            }
        });

        self.run();
    }

    fn run(&mut self) {
        let tick_dur = time::Duration::from_secs(1) / TICKS_PER_SECOND;
        let mut id = 0;

        loop {
            // println!("Starting tick {}", id);
            let start_time = time::Instant::now();

            self.tick();

            let elapsed_tick_dur = start_time.elapsed();
            if elapsed_tick_dur < tick_dur {
                thread::sleep(tick_dur - elapsed_tick_dur);
            } else {
                println!("Fully used tick time!!!");
            }

            id += 1;
        }
    }

    fn broadcast(&mut self, message: Message) {
        for stream in self.streams.values_mut() {
            stream.write_message(message.clone());
        }
    }

    fn tick(&mut self) {
        let new_names = {
            let mut jp = self.joining_players.lock().unwrap();
            let new_names = jp.iter().map(|p| p.1.clone()).collect::<Vec<String>>();
            for (stream, name) in jp.drain(..) {
                let id = self.game.add_player(Hero::John, name, Point::new(0.0, 0.0));
                self.streams.insert(id, stream);
            }
            new_names
        };

        for name in &new_names {
            self.broadcast(Message::ReceiveChat {
                user: "".into(),
                message: format!("{} has connected!", name),
            })
        }

        let mut commands = Vec::new();

        for player in self.game.players().into_iter().cloned().collect::<Vec<EntityID>>() {

            // self.world.modify_entity(player, |entity, data| {
            // });

            let mut stream = self.streams.get_mut(&player).unwrap();
            while let Some(message) = stream.try_get_message().unwrap() {
                match message {
                    Message::Ping { id } => {
                        match stream.write_message(Message::ReturnPing { id: id }) {
                            Ok(()) => {}
                            Err(err) => {
                                println!("Ping failed");
                                return;
                            }
                        }
                    }
                    Message::Quit {} => {
                        println!("Quit: {}",
                                 self.game
                                     .with_entity_data(player, |entity, data| {
                                         data.player[entity].name().to_string()
                                     })
                                     .unwrap())
                    }
                    Message::SendChat { message } => {}
                    Message::Command(command) => commands.push((command, player)),
                    _ => {}
                }
            }
        }

        for (command, id) in commands {
            self.game.run_command(command, id);
        }
    }
}

fn handle_client(mut stream: TcpStream,
                 joining_players: Arc<Mutex<Vec<(Stream, String)>>>)
                 -> io::Result<()> {
    println!("Connection from {}", stream.peer_addr().unwrap());
    let mut stream = common::Stream::new(stream);

    let m = stream.get_message().unwrap();
    let name = match m {
        Message::Connect { name } => {
            println!("Name: {}", name);
            name
        }
        _ => {
            println!("Client didn't send connect message, returning..");
            return Ok(());
        }
    };

    stream.write_message(Message::AcceptConnection { message: "Welcome to moba alpha.".into() })?;

    joining_players.lock().unwrap().push((stream, name));
    Ok(())

    // loop {
    // let message = match stream.get_message() {
    // Ok(message) => message,
    // Err(err) => {
    // println!("Client unexpectedly quit");
    // return Err(err);
    // }
    // };
    //
    // match message {
    // Message::Ping { id: id } => {
    // match stream.write_message(Message::ReturnPing { id: id }) {
    // Ok(()) => {}
    // Err(err) => {
    // println!("Ping failed");
    // return Ok(());
    // }
    // }
    // }
    // Message::Quit {} => println!("Quit: {}", name),
    // Message::SendChat { message } => {}
    // _ => {}
    // }
    // }
}
