use std::collections::{VecDeque, HashMap};
use std::ops::{Sub, Deref, DerefMut};
use na::{Point2, Vector2};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Hero {
    John,
}

impl Hero {
    pub fn radius(self) -> f64 {
        match self {
            Hero::John => 50.0,
        }
    }

    pub fn speed(self) -> f64 {
        match self {
            Hero::John => 100.0,
        }
    }
}

/// Has minimum of one state.
pub struct StateBuffer<T: Clone> {
    states: VecDeque<T>,
    num_states: usize,
}

impl<T: Clone> StateBuffer<T> {
    pub fn new(init: T, num_states: usize) -> Self {
        let mut states = VecDeque::with_capacity(num_states);
        states.push_front(init);
        StateBuffer {
            states: states,
            num_states: num_states,
        }
    }

    pub fn push(&mut self, state: T) -> Option<T> {
        self.states.push_front(state);
        if self.states.len() > self.num_states {
            self.states.pop_back()
        } else {
            None
        }
    }

    pub fn bump(&mut self) -> Option<T> {
        let latest_state = self.states[0].clone();
        self.push(latest_state)
    }
}

impl<T: Clone> Deref for StateBuffer<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.states[0]
    }
}

impl<T: Clone> DerefMut for StateBuffer<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.states[0]
    }
}


#[derive(Clone, Debug)]
pub enum Target {
    Nothing,
    Position(Point),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayerID(u32);

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub position: Point,
    target: Target,
}

pub struct Player {
    hero: Hero,
    id: PlayerID,
    name: String,
    pub state: StateBuffer<PlayerState>,
}

impl Player {
    pub fn hero(&self) -> Hero {
        self.hero
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> PlayerID {
        self.id
    }

    pub fn update(&mut self, t: f64) {
        match self.state.target {
            Target::Nothing => {}
            Target::Position(p) => {}
        }
    }
}

pub struct Game {
    players: Vec<Player>,
    next_player_id: u32,
}

impl Game {
    pub fn new() -> Self {
        Game {
            players: Vec::new(),
            next_player_id: 0,
        }
    }

    pub fn add_player(&mut self, hero: Hero, name: String, position: Point) -> PlayerID {
        let id = PlayerID(self.next_player_id);
        self.next_player_id += 1;
        let p = Player {
            hero: hero,
            id: id,
            name: name,
            state: StateBuffer::new(PlayerState {
                                        position: position,
                                        target: Target::Nothing,
                                    },
                                    2),
        };
        self.players.push(p);
        id
    }

    pub fn get_player(&mut self, id: PlayerID) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.id == id)
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn players_mut(&mut self) -> &mut [Player] {
        &mut self.players
    }

    pub fn run_command(&mut self, command: Command, origin: PlayerID) {
        let mut player = self.get_player(origin).unwrap();
        match command {
            Command::Move(target) => player.state.target = Target::Position(target),
        }
    }

    pub fn bump_state(&mut self) {
        for player in &mut self.players {
            player.state.bump();
        }
    }

    pub fn tick(&mut self, time: f64) {}
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Point { x: x, y: y }
    }
}

impl Sub<Point> for Point {
    type Output = Vector2<f64>;

    fn sub(self, right: Point) -> Self::Output {
        Point2::from(self) - Point2::from(right)
    }
}

impl From<Point> for Point2<f64> {
    fn from(p: Point) -> Self {
        Point2::new(p.x, p.y)
    }
}

impl From<Point2<f64>> for Point {
    fn from(p: Point2<f64>) -> Self {
        Point::new(p.x, p.y)
    }
}

impl From<Point> for Vector2<f64> {
    fn from(p: Point) -> Self {
        Vector2::new(p.x, p.y)
    }
}

impl From<Vector2<f64>> for Point {
    fn from(p: Vector2<f64>) -> Self {
        Point::new(p.x, p.y)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command {
    Move(Point),
}
