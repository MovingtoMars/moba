use std;
use std::net::{self, TcpStream};
use std::io;
use std::thread;
use std::time;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use piston_window::{self, Transformed, Window, Input, Button, MouseButton, Motion, Key};
use sdl2_window::Sdl2Window;

use common::*;

mod render;
use self::render::particle;

pub struct Client {
    name: String,
    team: Option<Team>,
    game: Game,
    viewport: render::Viewport,
    particles: Vec<Box<particle::Particle>>,
    id: Option<EntityID>,
    stream: Option<Stream>,
    game_mouse_x: f64,
    game_mouse_y: f64,
    screen_mouse_x: f64,
    screen_mouse_y: f64,
    selected_entity_id: Option<EntityID>,
    hovered_entity_id: Option<EntityID>,
}

impl Client {
    pub fn new(name: String, team: Option<Team>) -> Self {
        Client {
            name,
            team,

            game: Game::new(),
            viewport: render::Viewport::new(-500.0, -500.0, 1.0),
            particles: Vec::new(),
            id: None,
            stream: None,
            game_mouse_x: 0.0,
            game_mouse_y: 0.0,

            screen_mouse_x: 0.0,
            screen_mouse_y: 0.0,

            selected_entity_id: None,
            hovered_entity_id: None,
        }
    }

    fn run_command(&mut self, command: Command) {
        self.game.run_command(command.clone(), self.id.unwrap());
        self.stream
            .as_mut()
            .unwrap()
            .write_message(Message::Command(command))
            .unwrap();
    }

    fn run(
        &mut self,
        current_ping: Arc<Mutex<u64>>,
        events: Arc<Mutex<Vec<Event>>>,
        player_entity_id: Arc<Mutex<Option<EntityID>>>,
    ) -> io::Result<()> {

        let mut window: piston_window::PistonWindow<Sdl2Window> =
            piston_window::WindowSettings::new("moba", [1280, 720])
                .exit_on_esc(true)
                .samples(1)
                .vsync(true)
                .build()
                .unwrap();

        let mut fonts = render::Fonts::new(window.factory.clone());

        let mut last_render_time = time::Instant::now();

        while let Some(e) = window.next() {
            let piston_window::Size { width, height } = window.draw_size();

            if self.id.is_none() {
                if let Some(id) = *player_entity_id.lock().unwrap() {
                    self.id = Some(id);
                }
            }

            {
                let mut events_handle = events.lock().unwrap();
                for ev in events_handle.drain(..) {
                    self.game.run_event(ev);
                }
            }

            match e {
                Input::Render(_) => {
                    self.render(
                        &mut window,
                        &current_ping,
                        width,
                        &mut last_render_time,
                        e,
                        &mut fonts,
                    )
                }
                Input::Move(motion) => {
                    match motion {
                        Motion::MouseCursor(x, y) => self.handle_mouse_motion(x, y),
                        _ => {}
                    }
                }
                Input::Press(button) => {
                    match button {
                        Button::Mouse(mouse_button) => self.handle_mouse_press(mouse_button),
                        Button::Keyboard(key) => self.handle_keyboard_press(key),
                        _ => {}
                    }
                }
                _ => {}
            };
        }

        self.stream.as_mut().unwrap().write_message(Message::Quit)
    }

    fn render<W: piston_window::OpenGLWindow>(
        &mut self,
        window: &mut piston_window::PistonWindow<W>,
        current_ping: &Arc<Mutex<u64>>,
        width: u32,
        last_render_time: &mut time::Instant,
        e: Input,
        fonts: &mut render::Fonts,
    ) {
        // HACK
        if let Some(id) = self.id {
            self.game
                .with_component_mut::<Renderable, _, _>(id, |r| r.colour = [0.0, 0.0, 1.0, 1.0]);
        }

        let dur_since_last_render = last_render_time.elapsed();
        *last_render_time = time::Instant::now();

        // self.game.clone();
        // self.game.clone();
        // self.game.clone();

        window.draw_2d(&e, |c, g| {
            piston_window::clear([1.0; 4], g);
            // piston_window::rectangle([1.0, 0.0, 0.0, 1.0], // red
            //                          [0.0, 0.0, 100.0, 100.0],
            //                          c.transform,
            //

            {
                let viewport = self.viewport;
                for e in self.game.entity_ids_cloned() {
                    let e = self.game.get_entity(e).unwrap();
                    render::render(viewport, c, g, fonts, e, self.game.mut_world());
                }
            }

            for p in &mut self.particles {
                (&mut **p).render(self.viewport, c, g)
            }

            piston_window::text(
                [0.0, 0.0, 0.0, 1.0],
                14,
                &format!(
                    "Ping: {}",
                    std::cmp::min(*current_ping.lock().unwrap(), 999)
                ),
                &mut fonts.regular,
                c.transform.trans(width as f64 - 80.0, 15.0),
                g,
            );

            piston_window::text(
                [0.0, 0.0, 0.0, 1.0],
                14,
                &format!(
                    "Selected: {:?}    Hovered: {:?}",
                    self.selected_entity_id,
                    self.hovered_entity_id
                ),
                &mut fonts.regular,
                c.transform.trans(5.0, 15.0),
                g,
            );



            for p in &mut self.particles {
                p.update(
                    dur_since_last_render.as_secs() as f64 +
                        dur_since_last_render.subsec_nanos() as f64 / 1000000000.0,
                );
            }

            self.particles.retain(|p| !p.should_remove());
        });
    }

    fn handle_mouse_press(&mut self, mouse_button: MouseButton) {
        match mouse_button {
            MouseButton::Left => {
                self.selected_entity_id = self.entity_under_cursor();
            }
            MouseButton::Right => {
                let x = self.game_mouse_x;
                let y = self.game_mouse_y;
                if let Some(e) = self.targetable_entity_under_cursor() {
                    self.run_command(Command::SetTarget(Target::Entity(e)));
                } else {
                    self.run_command(Command::SetTarget(Target::Position(Point::new(x, y))));
                }
                self.particles
                    .push(Box::new(particle::RightClick::new(x, y)))
            }
            _ => {}
        }
    }

    fn handle_keyboard_press(&mut self, key: Key) {
        match key {
            Key::Q => {
                let command = Command::UseAbility {
                    ability_id: 0,
                    mouse_position: Some(Point::new(self.game_mouse_x, self.game_mouse_y)),
                };
                self.run_command(command);
            }

            _ => {}
        }
    }

    fn handle_mouse_motion(&mut self, x: f64, y: f64) {
        self.screen_mouse_x = x;
        self.screen_mouse_y = y;
        self.game_mouse_x = self.viewport.x_screen_to_game(x);
        self.game_mouse_y = self.viewport.y_screen_to_game(y);
        self.hovered_entity_id = self.entity_under_cursor();
    }

    fn entity_under_cursor(&mut self) -> Option<EntityID> {
        for e in self.game.entity_ids_cloned() {
            if self.game
                .entity_contains_point(e, self.game_mouse_x, self.game_mouse_y)
            {
                return Some(e);
            }
        }
        None
    }

    fn targetable_entity_under_cursor(&mut self) -> Option<EntityID> {
        for e in self.game.entity_ids_cloned() {
            let targetable = self.game.has_component::<Hitpoints>(e);
            if targetable &&
                self.game
                    .entity_contains_point(e, self.game_mouse_x, self.game_mouse_y)
            {
                return Some(e);
            }
        }
        None
    }

    pub fn connect(&mut self, addr: net::SocketAddrV4) -> io::Result<()> {
        let mut stream = Stream::new(TcpStream::connect(addr)?);
        self.stream = Some(stream.clone());

        let name = self.name.clone();
        let team = self.team;

        let current_ping = Arc::new(Mutex::new(0));
        let events = Arc::new(Mutex::new(Vec::new()));
        let player_entity_id = Arc::new(Mutex::new(None));

        {
            let current_ping = current_ping.clone();
            let events = events.clone();
            let player_entity_id = player_entity_id.clone();

            thread::spawn(move || {
                stream
                    .write_message(Message::Connect { name, team })
                    .expect("1");

                let message = stream.get_message().unwrap();
                match message {
                    Message::AcceptConnection { message } => {
                        println!("Connection successful: {}", message);
                    }
                    _ => {
                        panic!("Connection unsuccessful.");
                    }
                }

                let message = stream.get_message().unwrap();
                match message {
                    Message::SetPlayerEntityID(id) => *player_entity_id.lock().unwrap() = Some(id),
                    _ => panic!("Connection unsuccessful. (2)"),
                }

                let ping_store = Arc::new(Mutex::new(PingStore::new()));

                {
                    let mut stream = stream.clone();
                    let ping_store = ping_store.clone();
                    thread::spawn(move || loop {
                        stream
                            .write_message(Message::Ping {
                                id: ping_store.lock().unwrap().start_ping(),
                            })
                            .unwrap();
                        thread::sleep(time::Duration::from_secs(1));
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
                        Message::Events(mut e) => {
                            events.lock().unwrap().append(&mut e);
                            // let () = e;
                        }
                        _ => {}
                    }
                }
            });
        }

        self.run(current_ping, events, player_entity_id)
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
