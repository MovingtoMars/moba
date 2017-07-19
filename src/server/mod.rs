use std::net::{self, TcpListener, TcpStream};
use std::io;
use std::thread;
use std::time;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use common::{self, Message, Stream, Game, logic, Point, EntityID, Event, Team};

const TICKS_PER_SECOND: u32 = 60;

pub struct Server {
    game: Game,
    streams: HashMap<EntityID, Stream>,
    joining_players: Arc<Mutex<Vec<(Stream, String, Option<Team>)>>>,
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
            let addr = net::SocketAddrV4::new(net::Ipv4Addr::new(127, 0, 0, 1), port); // change to 0.0.0.0 to accept from all locations

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

            self.tick(1.0 / TICKS_PER_SECOND as f64);

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
            stream.write_message(message.clone()).unwrap();
        }
    }

    fn tick(&mut self, time: f64) {
        let new_names = {
            let jp = {
                let mut x = self.joining_players.lock().unwrap();
                let y = x.clone();
                x.clear();
                y
            };

            let new_names = jp.iter().map(|p| p.1.clone()).collect::<Vec<String>>();
            for (mut stream, name, team) in jp {
                let id = self.game.next_entity_id();
                let position = Point::new(0.0, 0.0);
                let hero = logic::HeroKind::John;
                stream
                    .write_message(Message::SetPlayerEntityID(id))
                    .unwrap();
                stream
                    .write_message(Message::Events(self.game.events_for_loading()))
                    .unwrap();
                self.game.add_player(id, hero, name.clone(), position, team);
                self.streams.insert(id, stream);
                self.broadcast(Message::Events(vec![
                    Event::AddHero {
                        id,
                        hero,
                        position,
                        name,
                        team,
                    },
                ]));

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
        let mut events = Vec::new();
        let mut players_to_remove = Vec::new();

        for player in self.game.players().to_owned() {
            let mut stream = self.streams.get_mut(&player).unwrap();
            while let Some(message) = stream.try_get_message() {
                if let Err(err) = message {
                    println!("Error from client stream: {:?}", err);
                    players_to_remove.push(player);
                    break;
                }

                match message.unwrap() {
                    Message::Ping { id } => {
                        stream
                            .write_message(Message::ReturnPing { id: id })
                            .unwrap()
                    }
                    Message::Quit {} => {
                        println!(
                            "Quit: {}",
                            self.game
                                .with_component::<common::Player, _, _>(
                                    player,
                                    |c| { c.name().to_string() }
                                )
                                .unwrap()
                        );

                        players_to_remove.push(player);
                        break;
                    }
                    Message::SendChat { message } => {}
                    Message::Command(command) => commands.push((command, player)),
                    _ => {}
                }
            }
        }

        for player in players_to_remove {
            self.streams.remove(&player);
            self.game.remove_entity(player);
            events.push(Event::RemoveEntity(player));
        }

        for (command, id) in commands {
            let es = self.game.run_command(command, id);
            self.game.run_events(&es);
            events.extend(es);
        }

        events.extend(self.game.tick(time));

        self.broadcast(Message::Events(events));
    }
}

fn handle_client(
    stream: TcpStream,
    joining_players: Arc<Mutex<Vec<(Stream, String, Option<Team>)>>>,
) -> io::Result<()> {
    println!("Connection from {}", stream.peer_addr().unwrap());
    let mut stream = common::Stream::new(stream);

    let m = stream.get_message().unwrap();
    let (name, team) = match m {
        Message::Connect { name, team } => {
            println!("Name: {}", name);
            (name, team)
        }
        _ => {
            println!("Client didn't send connect message, returning..");
            return Ok(());
        }
    };

    stream
        .write_message(Message::AcceptConnection {
            message: "Welcome to moba alpha.".into(),
        })?;

    joining_players.lock().unwrap().push((stream, name, team));
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
