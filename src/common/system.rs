use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use na::{Point2, Vector2};

use common::*;
use specs::{self, Join};

type RS<'a, T> = specs::ReadStorage<'a, T>;
type WS<'a, T> = specs::WriteStorage<'a, T>;

pub fn register_systems<'a, 'b>(d: specs::DispatcherBuilder<'a, 'b>) -> specs::DispatcherBuilder<'a, 'b> {
    let d = d.add(UpdateVelocitySystem, "UpdateVelocitySystem", &[]);
    let d = d.add(MotionSystem, "MotionSystem", &["UpdateVelocitySystem"]);
    let d = d.add_barrier();
    let d = d.add(BasicAttackerSystem, "BasicAttackerSystem", &[]);
    let d = d.add(ProjectileSystem, "ProjectileSystem", &[]);

    d
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

#[derive(SystemData)]
pub struct UpdateVelocityData<'a> {
        unitc: RS<'a, Unit>,
        velocityc: WS<'a, Velocity>,
        positionc: RS<'a, Position>,
        idc: RS<'a, EntityID>,
        playerc: RS<'a, Player>,
        hitpointsc: RS<'a, Hitpoints>,
        teamc: RS<'a, Team>,

        c: specs::Fetch<'a, Context>,
}

pub struct UpdateVelocitySystem;

impl<'a> specs::System<'a> for UpdateVelocitySystem {
    type SystemData = UpdateVelocityData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        let time = data.c.time;

        for (id, unit, velocity, position) in (&data.idc, &data.unitc, &mut data.velocityc, &data.positionc).join() {
            let speed = unit.speed;

            *velocity = match unit.target {
                Target::Nothing => Velocity::new(0.0, 0.0),
                Target::Position(p) => calculate_velocity(position.point, p, speed, time, 0.0),
                Target::Entity(e) => {
                    let e = data.c.get_entity(e).unwrap();
                    let target = data.positionc.get(e).unwrap();

                    let range = data.playerc
                        .get(data.c.get_entity(*id).unwrap())
                        .map(|player| player.hero.range())
                        .unwrap_or(0.0); // XXX replace with component::BasicAttacker

                    let attackable = data.hitpointsc.get(e).is_some();

                    let self_team = data.teamc.get(data.c.get_entity(*id).unwrap());
                    let target_team = data.teamc.get(e);
                    let attackable = attackable && (target_team == None || self_team != target_team);
                    let range = if attackable { range } else { 0.0 };

                    /// XXX: attackable component

                    calculate_velocity(position.point, target.point, speed, time, range)
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

#[derive(SystemData)]
pub struct MotionData<'a> {
        positionc: RS<'a, Position>,
        velocityc: RS<'a, Velocity>,
        idc: RS<'a, EntityID>,

        c: specs::Fetch<'a, Context>,
}

pub struct MotionSystem;

impl<'a> specs::System<'a> for MotionSystem {
    type SystemData = MotionData<'a>;

    fn run(&mut self, data: Self::SystemData) {
        for (&id, velocity, position) in (&data.idc, &data.velocityc, &data.positionc).join() {
            let dx = velocity.vector.x * data.c.time;
            let dy = velocity.vector.y * data.c.time;
            if dx.abs() < 0.1 && dy.abs() < 0.1 {
                continue;
            }
            let x = position.point.x + dx;
            let y = position.point.y + dy;

            let event = Event::EntityMove(id, Point::new(x, y));
            data.c.push_event(event);
        }
    }
}

#[derive(SystemData)]
pub struct BasicAttackerData<'a> {
        positionc: RS<'a, Position>,
        unitc: WS<'a, Unit>,

        c: specs::Fetch<'a, Context>,
}

pub struct BasicAttackerSystem;

impl<'a> specs::System<'a> for BasicAttackerSystem {
    type SystemData = BasicAttackerData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        for (position, mut unit) in (&data.positionc, &mut data.unitc).join() {
            if unit.attack_speed == 0.0 {
                continue;
            }

            assert!(unit.time_until_next_attack >= 0.0);

            if unit.time_until_next_attack > 0.0 {
                unit.time_until_next_attack -= data.c.time;
                unit.time_until_next_attack = unit.time_until_next_attack.max(0.0);
                continue;
            }

            match unit.target {
                Target::Entity(e) => {
                    unit.time_until_next_attack = 1.0 / unit.attack_speed;
                    data.c.push_event(Event::AddProjectile {
                        id: data.c.next_entity_id(),
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

#[derive(SystemData)]
pub struct ProjectileData<'a> {
        positionc: RS<'a, Position>,
        unitc: RS<'a, Unit>,
        idc: RS<'a, EntityID>,
        hitboxc: RS<'a, Hitbox>,
        projectilec: RS<'a, Projectile>,

        c: specs::Fetch<'a, Context>,
}

pub struct ProjectileSystem;

impl<'a> specs::System<'a> for ProjectileSystem {
    type SystemData = ProjectileData<'a>;

    fn run(&mut self, data: Self::SystemData) {
        for (id, position, unit, projectile, hitbox) in
            (&data.idc, &data.positionc, &data.unitc, &data.projectilec, &data.hitboxc).join()
        {
            //
            let target_entity_id = match unit.target {
                Target::Entity(e) => e,
                _ => continue,
            };
            let target_entity = data.c.get_entity(target_entity_id).unwrap();

            if data.hitboxc.get(target_entity).unwrap().contains_point(
                position.point.x,
                position.point.y,
                data.positionc.get(target_entity).unwrap().point.into(),
            ) {
                data.c.push_event(Event::DamageEntity {
                    id: target_entity_id,
                    damage: projectile.damage,
                });
                data.c.push_event(Event::RemoveEntity(*id));
            }
        }
    }
}
