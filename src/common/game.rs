use std::collections::{VecDeque, HashMap};
use std::ops::{Sub, Deref, DerefMut};
use na::{Point2, Vector2};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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


#[derive(Clone, Debug)]
pub enum Target {
    Nothing,
    Position(Point),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayerID(u32);

#[derive(Clone, Debug)]
pub struct Player {
    hero: Hero,
    id: PlayerID,
    name: String,
    pub position: Point,
    target: Target,
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
        match self.target {
            Target::Nothing => {}
            Target::Position(p) => {}
        }
    }
}

#[derive(Clone)]
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
            position: position,
            target: Target::Nothing,
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
            Command::Move(target) => player.target = Target::Position(target),
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
