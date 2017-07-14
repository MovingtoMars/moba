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

    pub fn damage(&mut self, damage: u16) {
        if damage > self.current {
            self.current = 0;
        } else {
            self.current -= damage;
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

    pub fn contains_point(&self, x: f64, y: f64, point: na::Point2<f64>) -> bool {
        use ncollide::query::PointQuery;
        self.shape.contains_point(
            &na::Isometry2::new(na::Vector2::new(x, y), na::zero()),
            &point,
        )
    }
}

#[derive(Clone, Debug)]
pub struct Projectile {
    pub damage: u16,
}

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

pub struct BasicAttacker {
    pub attack_speed: f64, // attacks_per_second
    pub time_until_next_attack: f64,
}

impl specs::Component for BasicAttacker {
    type Storage = specs::VecStorage<BasicAttacker>;
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
    pub vector: Vector,
}

impl Velocity {
    pub fn new(x: f64, y: f64) -> Self {
        Velocity { vector: Vector { x, y } }
    }
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
pub struct Team(pub u8);

impl specs::Component for Team {
    type Storage = specs::VecStorage<Team>;
}
