use std::sync::{Arc, Mutex};

use std::collections::HashMap;
use common::*;
use specs::{self, Join};
use ncollide::world::{CollisionWorld, CollisionGroups, GeometricQueryType};
use na::{Point2, Isometry2, Vector2};

pub type RS<'a, T> = specs::ReadStorage<'a, T>;
pub type WS<'a, T> = specs::WriteStorage<'a, T>;

pub fn register_systems<'a, 'b>(
    d: specs::DispatcherBuilder<'a, 'b>,
) -> specs::DispatcherBuilder<'a, 'b> {
    let d = d.add(UpdateVelocitySystem, "UpdateVelocitySystem", &[]);
    let d = d.add(MotionSystem, "MotionSystem", &["UpdateVelocitySystem"]);
    let d = d.add_barrier();

    let d = d.add(CollisionSystem, "CollisionSystem", &[]);
    let d = d.add_barrier();

    let d = d.add(BasicAttackerSystem, "BasicAttackerSystem", &[]);
    let d = d.add(ProjectileSystem, "ProjectileSystem", &[]); // XXX: race condition with BasicAttackerSystem?

    d
}

pub struct ContextInner {
    events: Vec<Event>,
    collisions: HashMap<EntityID, Vec<Collision>>, // TODO separate RWMutex
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
            inner: Arc::new(Mutex::new(ContextInner {
                events: Vec::new(),
                collisions: HashMap::new(),
            })),
            entity_map,
            next_entity_id,
        }
    }

    pub fn push_event(&self, event: Event) {
        self.inner.lock().unwrap().events.push(event);
    }

    pub fn push_collision(&self, collision: Collision) {
        let collisions = &mut self.inner.lock().unwrap().collisions;

        collisions
            .entry(collision.obj1)
            .or_insert_with(|| Vec::new())
            .push(collision);
        collisions
            .entry(collision.obj2)
            .or_insert_with(|| Vec::new())
            .push(collision.flip());
    }

    /// In all returned collisions, `this == collision.obj1`
    pub fn get_collisions(&self, this: EntityID, other: Option<EntityID>) -> Vec<Collision> {
        if Some(this) == other {
            // we don't allow self-collisions
            println!("Requested self-collisions!");
            return Vec::new();
        }

        if let Some(collisions) = self.inner.lock().unwrap().collisions.get(&this) {
            collisions
                .into_iter()
                .filter(|c| other.is_none() || other.unwrap() == c.obj2)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
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

        for (id, unit, velocity, position) in
            (&data.idc, &data.unitc, &mut data.velocityc, &data.positionc).join()
        {
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
    unitc: RS<'a, Unit>,
    teamc: RS<'a, Team>,
    idc: RS<'a, EntityID>,
    basic_attackerc: WS<'a, BasicAttacker>,

    c: specs::Fetch<'a, Context>,
}

pub struct BasicAttackerSystem;

impl<'a> specs::System<'a> for BasicAttackerSystem {
    type SystemData = BasicAttackerData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {

        for (&id, position, unit, mut basic_attacker) in
            (
                &data.idc,
                &data.positionc,
                &data.unitc,
                &mut data.basic_attackerc,
            ).join()
        {
            let entity = data.c.get_entity(id).unwrap();

            if basic_attacker.attack_speed == 0.0 {
                continue;
            }

            assert!(basic_attacker.time_until_next_attack >= 0.0);

            if basic_attacker.time_until_next_attack > 0.0 {
                basic_attacker.time_until_next_attack -= data.c.time;
                basic_attacker.time_until_next_attack =
                    basic_attacker.time_until_next_attack.max(0.0);
                continue;
            }

            match unit.target {
                Target::Entity(e) => {
                    basic_attacker.time_until_next_attack = 1.0 / basic_attacker.attack_speed;
                    data.c.push_event(Event::AddProjectile {
                        id: data.c.next_entity_id(),
                        position: position.point,
                        target: Target::Entity(e),
                        damage: 5,
                        team: data.teamc.get(entity).cloned(),
                        owner: id,
                    })
                }
                _ => {}
            }
        }
    }
}

#[derive(SystemData)]
pub struct ProjectileData<'a> {
    unitc: RS<'a, Unit>,
    idc: RS<'a, EntityID>,
    projectilec: RS<'a, Projectile>,
    teamc: RS<'a, Team>,
    hitpointsc: RS<'a, Hitpoints>,

    c: specs::Fetch<'a, Context>,
}

pub struct ProjectileSystem;

impl<'a> specs::System<'a> for ProjectileSystem {
    type SystemData = ProjectileData<'a>;

    fn run(&mut self, data: Self::SystemData) {
        for (&id, unit, projectile) in (&data.idc, &data.unitc, &data.projectilec).join() {
            let target_entity_id = match unit.target {
                Target::Entity(e) => Some(e),
                _ => None,
            };

            for collision in &data.c.get_collisions(id, target_entity_id) {
                if projectile.owner == collision.obj2 {
                    continue;
                }

                if logic::can_attack(
                    data.c.get_entity(id).unwrap(),
                    data.c.get_entity(collision.obj2).unwrap(),
                    &data.teamc,
                    &data.hitpointsc,
                ) {
                    data.c.push_event(Event::DamageEntity {
                        id: collision.obj2,
                        damage: projectile.damage,
                    });
                    data.c.push_event(Event::RemoveEntity(id));
                    break;
                } else if target_entity_id.is_some() {
                    eprintln!("E1");
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct Collision {
    pub obj1: EntityID,
    pub obj2: EntityID,
    pub normal: Vector2<f64>,
    pub depth: f64,
}

impl Collision {
    pub fn flip(mut self) -> Self {
        use std;
        std::mem::swap(&mut self.obj1, &mut self.obj2);

        self.normal = self.normal * -1.0; // XXX is this right?

        self
    }
}

#[derive(SystemData)]
pub struct CollisionData<'a> {
    hitboxc: RS<'a, Hitbox>,
    idc: RS<'a, EntityID>,
    positionc: RS<'a, Position>,

    c: specs::Fetch<'a, Context>,
}

pub struct CollisionSystem;

impl<'a> specs::System<'a> for CollisionSystem {
    type SystemData = CollisionData<'a>;

    fn run(&mut self, data: Self::SystemData) {
        let mut collision_world: CollisionWorld<Point2<f64>, Isometry2<f64>, ()> =
            CollisionWorld::new(0.1, false);

        for (id, hitbox, position) in (&data.idc, &data.hitboxc, &data.positionc).join() {
            collision_world.deferred_add(
                id.0 as usize,
                position.point.into_isometry(0.0),
                hitbox.shape_handle.clone(),
                CollisionGroups::new(),
                GeometricQueryType::Contacts(0.0),
                (),
            );
        }

        collision_world.update();

        for (obj1, obj2, contact) in collision_world.contacts() {
            let collision = Collision {
                obj1: EntityID(obj1.uid as u32),
                obj2: EntityID(obj2.uid as u32),
                depth: contact.depth,
                normal: contact.normal,
            };

            data.c.push_collision(collision);
        }
    }
}
