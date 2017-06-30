use std::cmp;

use specs;
use common::*;
use ncollide;
use na;

type Shape = ncollide::shape::Shape<na::Point2<f64>, na::Isometry2<f64>>;

#[derive(Debug, Clone)]
pub struct Hitpoints {
    max: u16,
    current: u16,
}

impl Hitpoints {
    pub fn new_at_max(max: u16) -> Self {
        Hitpoints {
            max: max,
            current: max,
        }
    }

    pub fn max(&self) -> u16 {
        self.max
    }

    pub fn current(&self) -> u16 {
        self.current
    }

    pub fn set_current(&mut self, to: u16) {
        self.current = cmp::min(self.max, to)
    }

    pub fn set_max(&mut self, to: u16) {
        let diff = if to > self.max { to - self.max } else { 0 };
        self.max = to;
        self.current += diff;
        if self.current > self.max {
            self.current = self.max;
        }
    }
}

impl specs::Component for Hitpoints {
    type Storage = specs::VecStorage<Hitpoints>;
}

#[derive(Debug, Clone)]
pub struct Position {
    pub point: Point,
}

impl specs::Component for Position {
    type Storage = specs::VecStorage<Position>;
}

pub struct Hitbox {
    pub shape: Box<Shape>,
}

impl specs::Component for Hitbox {
    type Storage = specs::VecStorage<Hitbox>;
}

impl Hitbox {
    pub fn new<S: ncollide::shape::Shape<na::Point2<f64>, na::Isometry2<f64>>>(shape: S) -> Self {
        Hitbox { shape: Box::new(shape) }
    }

    pub fn new_ball(radius: f64) -> Self {
        Hitbox::new(ncollide::shape::Ball::new(radius))
    }
}

#[derive(Clone, Debug)]
pub struct Projectile {}

impl specs::Component for Projectile {
    type Storage = specs::HashMapStorage<Projectile>;
}

#[derive(Clone, Debug)]
pub struct Player {
    pub hero: Hero,
    pub name: String,
}

impl specs::Component for Player {
    type Storage = specs::HashMapStorage<Player>;
}

impl Player {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug)]
pub struct Renderable {
    pub radius: f64,
    pub colour: [f32; 4],
}

impl specs::Component for Renderable {
    type Storage = specs::VecStorage<Renderable>;
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub speed: f64,
    pub target: Target,
}

impl specs::Component for Unit {
    type Storage = specs::VecStorage<Unit>;
}

#[derive(Clone, Debug, Default)]
pub struct Velocity {
    pub x: f64,
    pub y: f64,
}

impl specs::Component for Velocity {
    type Storage = specs::VecStorage<Velocity>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    Hero,
    Projectile,
}

impl specs::Component for EntityKind {
    type Storage = specs::VecStorage<EntityKind>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityID(pub u32);

impl specs::Component for EntityID {
    type Storage = specs::VecStorage<EntityID>;
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// 0 means unaligned (so can be attacked by anyone).
/// TODO remove?
pub struct Team(pub u8);

impl Team {
    pub fn is_unaligned(self) -> bool {
        self.0 == 0
    }
}

impl specs::Component for Team {
    type Storage = specs::VecStorage<Team>;
}
