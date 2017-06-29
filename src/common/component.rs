use specs;
use common::*;
use ncollide;
use na;

type Shape = ncollide::shape::Shape<na::Point2<f64>, na::Isometry2<f64>>;

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
    pub target: Target,
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

#[derive(Clone, Debug)]
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
