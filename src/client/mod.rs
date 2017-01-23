use std;
use std::net::{self, TcpStream};
use std::io;
use std::thread;
use std::time;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use piston_window::{self, Transformed, Window, Event, Input, Button, MouseButton,
                    MouseCursorEvent, Motion};

use common::{self, Stream, Message, Game, Player, Hero, Point, Command, EntityID};

mod render;
use self::render::particle;

pub struct Client {
    name: String,
    game: Game,
    viewport: render::Viewport,
    particles: Vec<Box<particle::Particle>>,
    id: Option<EntityID>,
    stream: Option<Stream>,
    pending_commands: Vec<Command>,
    game_mouse_x: f64,
    game_mouse_y: f64,
    screen_mouse_x: f64,
    screen_mouse_y: f64,
}

impl Client {
    pub fn new(name: String) -> Self {
        Client {
            name: name,
            game: Game::new(),
            viewport: render::Viewport::new(-500.0, -500.0, 1.0),
            particles: Vec::new(),
            id: None,
            stream: None,
            pending_commands: Vec::new(),
            game_mouse_x: 0.0,
            game_mouse_y: 0.0,

            screen_mouse_x: 0.0,
            screen_mouse_y: 0.0,
        }
    }

    fn run_command(&mut self, command: Command) {
        self.game.run_command(command.clone(), self.id.unwrap());
        self.stream.as_mut().unwrap().write_message(Message::Command(command)).unwrap();
    }

    fn run(&mut self, current_ping: Arc<Mutex<u64>>) -> io::Result<()> {
        self.id = Some(self.game.add_player(Hero::John, self.name.clone(), Point::new(0.0, 0.0)));

        let mut window: piston_window::PistonWindow =
            piston_window::WindowSettings::new("moba", [1280, 720])
                .exit_on_esc(true)
                .samples(4)
                .vsync(true)
                .build()
                .unwrap();

        let mut glyphs =
            piston_window::Glyphs::new("./assets/fonts/NotoSans-unhinted/NotoSans-Regular.ttf",
                                       window.factory.clone())
                .unwrap();

        let mut last_render_time = time::Instant::now();

        while let Some(e) = window.next() {
            let piston_window::Size { width, height } = window.draw_size();

            match e {
                Event::Render(_) => {
                    let dur_since_last_render = last_render_time.elapsed();
                    last_render_time = time::Instant::now();

                    window.draw_2d(&e, |c, g| {
                        piston_window::clear([1.0; 4], g);
                        // piston_window::rectangle([1.0, 0.0, 0.0, 1.0], // red
                        //                          [0.0, 0.0, 100.0, 100.0],
                        //                          c.transform,
                        //

                        {
                            // well this looks a bit bad zzz
                            let entities = self.game
                                .entity_ids()
                                .into_iter()
                                .cloned()
                                .collect::<Vec<EntityID>>();
                            let viewport = self.viewport;
                            for e in entities {
                                self.game.with_entity_data(e, |entity, data| {
                                    render::render(viewport, c, g, entity, data);
                                });
                            }
                        }

                        for p in &mut self.particles {
                            (&mut **p).render(self.viewport, c, g)
                        }

                        piston_window::text([0.0, 0.0, 0.0, 1.0],
                                            14,
                                            &format!("Ping: {}",
                                                     std::cmp::min(*current_ping.lock().unwrap(),
                                                                   999)),
                                            &mut glyphs,
                                            c.transform.trans(width as f64 - 80.0, 15.0),
                                            g);



                        for p in &mut self.particles {
                            p.update(dur_since_last_render.as_secs() as f64 +
                                     dur_since_last_render.subsec_nanos() as f64 / 1000000000.0);
                        }

                        self.particles.retain(|p| !p.should_remove());
                    });
                }

                Event::Input(input) => self.handle_input(input),

                _ => {}
            };
        }

        Ok(())
    }

    fn handle_input(&mut self, input: Input) {
        match input {
            Input::Move(motion) => {
                match motion {
                    Motion::MouseCursor(x, y) => {
                        self.screen_mouse_x = x;
                        self.screen_mouse_y = y;
                        self.game_mouse_x = self.viewport.x_screen_to_game(x);
                        self.game_mouse_y = self.viewport.y_screen_to_game(y);
                    }
                    _ => {}
                }
            }
            Input::Press(button) => {
                match button {
                    Button::Mouse(mouse_button) => {
                        match mouse_button {
                            MouseButton::Right => {
                                let x = self.game_mouse_x;
                                let y = self.game_mouse_y;
                                self.run_command(Command::Move(Point::new(x, y)));
                                self.particles
                                    .push(Box::new(particle::RightClick::new(x, y)))
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn connect(&mut self, addr: net::SocketAddrV4) -> io::Result<()> {
        let mut stream = Stream::new(TcpStream::connect(addr)?);
        self.stream = Some(stream.clone());

        let name = self.name.clone();

        let mut current_ping = Arc::new(Mutex::new(0));

        {
            let current_ping = current_ping.clone();

            thread::spawn(move || {
                stream.write_message(Message::Connect { name: name }).expect("1");

                let message = stream.get_message().unwrap();
                match message {
                    Message::AcceptConnection { message } => {
                        println!("Connection successful: {}", message);

                    }
                    _ => {
                        println!("Connection unsuccessful.");
                        return;
                    }
                }

                let mut ping_store = Arc::new(Mutex::new(PingStore::new()));

                {
                    let mut stream = stream.clone();
                    let mut ping_store = ping_store.clone();
                    thread::spawn(move || {
                        loop {
                            stream.write_message(Message::Ping {
                                id: ping_store.lock().unwrap().start_ping(),
                            });
                            thread::sleep(time::Duration::from_secs(1));
                        }
                    });
                }

                loop {
                    let message = stream.get_message().unwrap();
                    match message {
                        Message::Kick { reason } => {
                            println!("Kicked: {}", reason);
                            break;
                        }
                        Message::ReturnPing { id } => {
                            let dur = ping_store.lock().unwrap().end_ping(id).unwrap();
                            let ping_ms = dur.as_secs() * 1000 +
                                          (dur.subsec_nanos() / 1000000) as u64;
                            *current_ping.lock().unwrap() = ping_ms;
                            // println!("Ping: {}ms", ping_ms);
                        }
                        Message::ReceiveChat { user, message } => {
                            if user != "" {
                                print!("[{}] ", user);
                            }
                            println!("{}", message);
                        }
                        _ => {}
                    }
                }
            });
        }

        self.run(current_ping)
    }
}

struct PingStore {
    next_id: u64,
    sent_times: HashMap<u64, time::Instant>,
}

impl PingStore {
    fn new() -> Self {
        PingStore {
            next_id: 0,
            sent_times: HashMap::new(),
        }
    }

    fn next_id(&mut self) -> u64 {
        let t = self.next_id;
        self.next_id += 1;
        t
    }

    fn start_ping(&mut self) -> u64 {
        let id = self.next_id();

        self.sent_times.insert(id, time::Instant::now());
        id
    }

    fn end_ping(&mut self, id: u64) -> Option<time::Duration> {
        match self.sent_times.remove(&id) {
            Some(sent_time) => Some(sent_time.elapsed()),
            None => None,
        }
    }
}
