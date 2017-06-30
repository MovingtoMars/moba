use std::sync::Mutex;
use std::collections::HashMap;

use common::*;
use specs::{self, Join};

pub struct ContextInner {
    events: Vec<Event>,
}

#[derive(Clone)]
pub struct Context {
    time: f64,
    inner: Arc<Mutex<ContextInner>>,
    entity_map: Arc<Mutex<HashMap<EntityID, specs::Entity>>>, // should be read only
}

impl Context {
    pub fn new(time: f64, entity_map: Arc<Mutex<HashMap<EntityID, specs::Entity>>>) -> Self {
        Context {
            time,
            inner: Arc::new(Mutex::new(ContextInner { events: Vec::new() })),
            entity_map,
        }
    }

    pub fn push_event(&self, event: Event) {
        self.inner.lock().unwrap().events.push(event);
    }

    pub fn events(&self) -> Vec<Event> {
        self.inner.lock().unwrap().events.clone()
    }

    pub fn get_entity(&self, id: EntityID) -> Option<specs::Entity> {
        self.entity_map.lock().unwrap().get(&id).cloned()
    }
}

pub struct UpdateVelocitySystem;

impl specs::System<Context> for UpdateVelocitySystem {
    fn run(&mut self, arg: specs::RunArg, c: Context) {
        let (unitc, mut velocityc, positionc, idc, playerc) = arg.fetch(|w| {
            (
                w.read::<Unit>(),
                w.write::<Velocity>(),
                w.read::<Position>(),
                w.read::<EntityID>(),
                w.read::<Player>(),
            )
        });

        for (unit, velocity, position) in (&unitc, &mut velocityc, &positionc).iter() {
            let mut speed = unit.speed;

            *velocity = match unit.target {
                Target::Nothing => Velocity { x: 0.0, y: 0.0 },
                Target::Position(p) => calculate_velocity(position.point, p, speed, c.time),
                Target::Entity(e) => {
                    let target = positionc.get(c.get_entity(e).unwrap()).unwrap();
                    calculate_velocity(position.point, target.point, speed, c.time)
                }
            };
        }
    }
}

fn calculate_velocity(source: Point, target: Point, mut speed: f64, time: f64) -> Velocity {
    let dx = target.x - source.x;
    let dy = target.y - source.y;
    let d = (dx * dx + dy * dy).sqrt();
    if d == 0.0 {
        return Velocity::default();
    }
    if speed * time > d {
        speed = d;
    }
    let ratio = speed / d;

    Velocity {
        x: ratio * dx,
        y: ratio * dy,
    }
}

pub struct MotionSystem;

impl specs::System<Context> for MotionSystem {
    fn run(&mut self, arg: specs::RunArg, c: Context) {
        let (idc, velocityc, mut positionc) = arg.fetch(|w| {
            (
                w.read::<EntityID>(),
                w.read::<Velocity>(),
                w.write::<Position>(),
            )
        });

        for (&id, velocity, mut position) in (&idc, &velocityc, &mut positionc).iter() {
            let dx = velocity.x * c.time;
            let dy = velocity.y * c.time;
            if dx.abs() < 0.1 && dy.abs() < 0.1 {
                continue;
            }
            let x = position.point.x + dx;
            let y = position.point.y + dy;

            let event = Event::EntityMove(id, Point::new(x, y));
            c.push_event(event);
        }
    }
}
