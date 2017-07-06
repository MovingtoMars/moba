use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use na::{Point2, Vector2};

use common::*;
use specs::{self, Join};

pub fn register_systems(planner: &mut specs::Planner<Context>) {
    planner.add_system(UpdateVelocitySystem, "UpdateVelocitySystem", 100);
    planner.add_system(MotionSystem, "MotionSystem", 99);
    planner.add_system(BasicAttackerSystem, "BasicAttackerSystem", 98);
    planner.add_system(ProjectileSystem, "ProjectileSystem", 97);
}

pub struct ContextInner {
    events: Vec<Event>,
}

#[derive(Clone)]
pub struct Context {
    time: f64,
    inner: Arc<Mutex<ContextInner>>,
    entity_map: Arc<Mutex<HashMap<EntityID, specs::Entity>>>, // should be read only
    next_entity_id: Arc<Mutex<u32>>,
}

impl Context {
    pub fn new(
        time: f64,
        entity_map: Arc<Mutex<HashMap<EntityID, specs::Entity>>>,
        next_entity_id: Arc<Mutex<u32>>,
    ) -> Self {
        Context {
            time,
            inner: Arc::new(Mutex::new(ContextInner { events: Vec::new() })),
            entity_map,
            next_entity_id,
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

    // XXX don't duplicate this method with Game
    pub fn next_entity_id(&self) -> EntityID {
        let mut next_entity_id = self.next_entity_id.lock().unwrap();
        let t = *next_entity_id;
        *next_entity_id = next_entity_id.checked_add(1).unwrap();
        EntityID(t)
    }
}

pub struct UpdateVelocitySystem;

impl specs::System<Context> for UpdateVelocitySystem {
    fn run(&mut self, arg: specs::RunArg, c: Context) {
        let (unitc, mut velocityc, positionc, idc, playerc, hitpointsc, teamc) = arg.fetch(|w| {
            (
                w.read::<Unit>(),
                w.write::<Velocity>(),
                w.read::<Position>(),
                w.read::<EntityID>(),
                w.read::<Player>(),
                w.read::<Hitpoints>(),
                w.read::<Team>(),
            )
        });

        for (id, unit, velocity, position) in (&idc, &unitc, &mut velocityc, &positionc).iter() {
            let speed = unit.speed;

            *velocity = match unit.target {
                Target::Nothing => Velocity::new(0.0, 0.0),
                Target::Position(p) => calculate_velocity(position.point, p, speed, c.time, 0.0),
                Target::Entity(e) => {
                    let e = c.get_entity(e).unwrap();
                    let target = positionc.get(e).unwrap();

                    let range = playerc
                        .get(c.get_entity(*id).unwrap())
                        .map(|player| player.hero.range())
                        .unwrap_or(0.0); // XXX replace with component::BasicAttacker

                    let attackable = hitpointsc.get(e).is_some();

                    let self_team = teamc.get(c.get_entity(*id).unwrap());
                    let target_team = teamc.get(e);
                    let attackable = attackable && (target_team == None || self_team != target_team);
                    let range = if attackable { range } else { 0.0 };

                    /// XXX: attackable component

                    calculate_velocity(position.point, target.point, speed, c.time, range)
                }
            };
        }
    }
}

fn calculate_velocity(
    source: Point,
    target: Point,
    mut speed: f64,
    time: f64,
    range: f64,
) -> Velocity {
    let mut vector = target - source;
    let mut d = vector.norm();

    if d - range > 0.0 {
        vector = vector.with_norm(d - range);
        d = vector.norm();
    } else {
        return Velocity::default();
    }

    if speed * time > d {
        speed = d;
    }
    let ratio = speed / d;

    Velocity { vector: vector * ratio }
}

pub struct MotionSystem;

impl specs::System<Context> for MotionSystem {
    fn run(&mut self, arg: specs::RunArg, c: Context) {
        let (idc, velocityc, positionc) = arg.fetch(|w| {
            (
                w.read::<EntityID>(),
                w.read::<Velocity>(),
                w.read::<Position>(),
            )
        });

        for (&id, velocity, position) in (&idc, &velocityc, &positionc).iter() {
            let dx = velocity.vector.x * c.time;
            let dy = velocity.vector.y * c.time;
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

pub struct BasicAttackerSystem;

impl specs::System<Context> for BasicAttackerSystem {
    fn run(&mut self, arg: specs::RunArg, c: Context) {
        let (idc, playerc, positionc, mut unitc) = arg.fetch(|w| {
            (
                w.read::<EntityID>(),
                w.read::<Player>(),
                w.read::<Position>(),
                w.write::<Unit>(),
            )
        });

        for (id, player, position, mut unit) in (&idc, &playerc, &positionc, &mut unitc).iter() {
            assert!(unit.time_until_next_attack >= 0.0);

            if unit.time_until_next_attack > 0.0 {
                unit.time_until_next_attack -= c.time;
                unit.time_until_next_attack = unit.time_until_next_attack.max(0.0);
                continue;
            }

            match unit.target {
                Target::Entity(e) => {
                    unit.time_until_next_attack = 1.0 / unit.attack_speed;
                    c.push_event(Event::AddProjectile {
                        id: c.next_entity_id(),
                        position: position.point,
                        target: Target::Entity(e),
                        damage: 5,
                    })
                }
                _ => {}
            }
        }
    }
}

pub struct ProjectileSystem;

impl specs::System<Context> for ProjectileSystem {
    fn run(&mut self, arg: specs::RunArg, c: Context) {
        let (idc, projectilec, positionc, hitboxc, unitc) = arg.fetch(|w| {
            (
                w.read::<EntityID>(),
                w.read::<Projectile>(),
                w.read::<Position>(),
                w.read::<Hitbox>(),
                w.read::<Unit>(),
            )
        });



        for (id, position, unit, projectile, hitbox) in
            (&idc, &positionc, &unitc, &projectilec, &hitboxc).iter()
        {
            //
            let target_entity_id = match unit.target {
                Target::Entity(e) => e,
                _ => continue,
            };
            let target_entity = c.get_entity(target_entity_id).unwrap();

            if hitboxc.get(target_entity).unwrap().contains_point(
                position.point.x,
                position.point.y,
                positionc.get(target_entity).unwrap().point.into(),
            ) {
                c.push_event(Event::DamageEntity {
                    id: target_entity_id,
                    damage: projectile.damage,
                });
                c.push_event(Event::RemoveEntity(*id));
            }
        }
    }
}
