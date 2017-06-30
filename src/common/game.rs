use std::collections::HashMap;
use std::ops::Sub;
use std::sync::{Arc, Mutex};
use na::{self, Point2, Vector2};
use ncollide::query::PointQuery;
use specs;

use common::*;

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
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
            Hero::John => 200.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Target {
    Nothing,
    Position(Point),
    Entity(EntityID),
}

pub struct Game {
    entity_ids: Vec<EntityID>,
    players: Vec<EntityID>,
    next_entity_id: u32,
    entity_map: Arc<Mutex<HashMap<EntityID, specs::Entity>>>,
    planner: specs::Planner<Context>,
}

impl Game {
    pub fn new() -> Self {
        let mut w = specs::World::new();
        w.register::<EntityID>();
        w.register::<EntityKind>();
        w.register::<Position>();
        w.register::<Player>();
        w.register::<Unit>();
        w.register::<Velocity>();
        w.register::<Renderable>();
        w.register::<Hitbox>();
        w.register::<Projectile>();
        w.register::<Hitpoints>();
        w.register::<Team>();

        let mut planner = specs::Planner::new(w, 4);

        planner.add_system(UpdateVelocitySystem, "UpdateVelocitySystem", 100);
        planner.add_system(MotionSystem, "MotionSystem", 99);

        Game {
            entity_ids: Vec::new(),
            players: Vec::new(),
            next_entity_id: 0,
            entity_map: Arc::new(Mutex::new(HashMap::new())),
            planner: planner,
        }
    }

    pub fn remove_player(&mut self, id: EntityID) -> Event {
        if !self.players.contains(&id) {
            panic!("removed non-existent player");
        }

        self.players.retain(|&p| p != id);

        self.remove_entity(id)
    }

    pub fn remove_entity(&mut self, id: EntityID) -> Event {
        self.entity_ids.retain(|&x| x != id);
        let e = self.entity_map.lock().unwrap().remove(&id).unwrap();
        self.planner.mut_world().delete_later(e);

        Event::RemoveEntity(id)
    }

    pub fn players(&self) -> &[EntityID] {
        &self.players
    }

    pub fn next_entity_id(&mut self) -> EntityID {
        let t = self.next_entity_id;
        self.next_entity_id = self.next_entity_id.checked_add(1).unwrap();
        EntityID(t)
    }

    pub fn add_player(
        &mut self,
        id: EntityID,
        hero: Hero,
        name: String,
        position: Point,
    ) -> EntityID {
        let e = self.add_entity(id, EntityKind::Hero, |entity| {
            entity
                .with(Position { point: position })
                .with(Player {
                    hero: hero,
                    name: name,
                })
                .with(Renderable {
                    radius: hero.radius(),
                    colour: [0.0, 1.0, 0.0, 1.0],
                })
                .with(Hitbox::new_ball(hero.radius()))
                .with(Unit {
                    speed: hero.speed(),
                    target: Target::Nothing,
                })
                .with(Hitpoints::new_at_max(50))
                .with(Velocity { x: 0.0, y: 0.0 })
        });

        self.players.push(e);
        e
    }

    pub fn add_projectile(&mut self, id: EntityID, position: Point, target: Target) -> EntityID {
        let e = self.add_entity(id, EntityKind::Projectile, |entity| {
            entity
                .with(Position { point: position })
                .with(Projectile {})
                .with(Renderable {
                    radius: 5.0,
                    colour: [1.0, 0.0, 0.0, 1.0],
                })
                .with(Hitbox::new_ball(5.0))
                .with(Unit {
                    speed: 400.0,
                    target: target,
                })
                .with(Velocity { x: 0.0, y: 0.0 })
        });
        e
    }

    pub fn add_entity<F>(&mut self, id: EntityID, kind: EntityKind, f: F) -> EntityID
    where
        F: FnOnce(specs::EntityBuilder<()>) -> specs::EntityBuilder<()>,
    {
        let entity = self.planner.mut_world().create_now();
        let entity = entity.with(kind).with(id);
        let entity = f(entity);
        let entity = entity.build();

        self.entity_map.lock().unwrap().insert(id, entity);
        self.entity_ids.push(id);

        id
    }

    pub fn get_entity(&self, id: EntityID) -> Option<specs::Entity> {
        self.entity_map.lock().unwrap().get(&id).cloned()
    }

    pub fn entity_ids(&self) -> &[EntityID] {
        &self.entity_ids
    }

    // TODO: don't need this fn?
    pub fn entity_ids_cloned(&self) -> Vec<EntityID> {
        self.entity_ids.clone()
    }

    pub fn run_custom<F>(&mut self, f: F)
    where
        F: 'static + Send + FnOnce(specs::RunArg),
    {
        self.planner.run_custom(f)
    }

    pub fn mut_world(&mut self) -> &mut specs::World {
        self.planner.mut_world()
    }

    pub fn with_component<T: specs::Component, U, F>(&mut self, e: EntityID, f: F) -> Option<U>
    where
        F: FnOnce(&T) -> U,
    {
        let entity = match self.get_entity(e) {
            Some(entity) => entity,
            None => return None,
        };

        let world = self.planner.mut_world();
        let storage = world.read::<T>();
        let component = match storage.get(entity) {
            Some(x) => x,
            None => return None,
        };

        Some(f(component))
    }

    pub fn with_component_mut<T: specs::Component, U, F>(&mut self, e: EntityID, f: F) -> Option<U>
    where
        F: FnOnce(&mut T) -> U,
    {
        let entity = match self.get_entity(e) {
            Some(entity) => entity,
            None => return None,
        };

        let world = self.planner.mut_world();
        let mut storage = world.write::<T>();
        let component = match storage.get_mut(entity) {
            Some(x) => x,
            None => return None,
        };

        Some(f(component))
    }

    pub fn clone_component<T: specs::Component + Clone>(&mut self, e: EntityID) -> Option<T> {
        self.with_component::<T, _, _>(e, |c| c.clone())
    }

    pub fn has_component<T: specs::Component>(&mut self, e: EntityID) -> bool {
        self.with_component::<T, _, _>(e, |_| {}).is_some()
    }

    pub fn entity_contains_point(&mut self, e: EntityID, x: f64, y: f64) -> bool {
        let entity = match self.get_entity(e) {
            Some(entity) => entity,
            None => return false,
        };

        let world = self.planner.mut_world();
        let hb_storage = world.read::<Hitbox>();
        let hb = match hb_storage.get(entity) {
            Some(x) => x,
            None => return false,
        };
        let pos_storage = world.read::<Position>();
        let pos = match pos_storage.get(entity) {
            Some(x) => x,
            None => return false,
        };

        let hb_pos = na::Isometry2::new(pos.point.into(), na::zero());

        hb.shape.contains_point(&hb_pos, &Point2::new(x, y))
    }

    pub fn run_command(&mut self, command: Command, origin: EntityID) {
        let entity = self.get_entity(origin).unwrap();

        // self.world.modify_entity(entity, |entity: ecs::ModifyData<MyComponents>,
        //                           data: &mut MyComponents| {
        //     match command {
        //         Command::Move(target) => data.player[entity].target = Target::Position(target),
        //     }
        // });

        match command {
            Command::SetTarget(target) => {
                self.run_custom(move |arg| {
                    let mut tc = arg.fetch(|world| world.write::<Unit>());

                    tc.get_mut(entity).unwrap().target = match target {
                        Target::Entity(id) if id == origin => Target::Nothing, // XXX error?
                        x => x,
                    };
                });
            }
        }
    }

    pub fn run_event(&mut self, event: Event) {
        println!("{:?}", event);
        match event {
            Event::RemoveEntity(id) => {
                let e = self.get_entity(id).unwrap();
                self.planner.mut_world().delete_later(e);
            }
            Event::EntityMove(id, point) => {
                let e = self.get_entity(id).unwrap();
                self.run_custom(move |arg| {
                    let mut posc = arg.fetch(|w| w.write::<Position>());
                    posc.get_mut(e).unwrap().point = point;
                });
            }
            Event::AddHero {
                id,
                hero,
                position,
                name,
            } => {
                self.add_player(id, hero, name, position);
            }
            Event::AddProjectile {
                id,
                position,
                target,
            } => {
                self.add_projectile(id, position, target);
            }
        }
    }

    pub fn run_events(&mut self, events: &[Event]) {
        for e in events {
            self.run_event(e.clone());
        }
    }

    pub fn tick(&mut self, time: f64) -> Vec<Event> {
        let context = Context::new(time, self.entity_map.clone());
        self.planner.dispatch(context.clone());
        self.planner.wait();

        let events = context.events();

        self.run_events(&events);
        events
    }

    pub fn events_for_loading(&mut self) -> Vec<Event> {
        let mut events = Vec::new();

        for &id in &self.entity_ids {
            let e = self.get_entity(id).unwrap();
            let world = self.planner.mut_world();
            let kind = *world.read::<EntityKind>().get(e).unwrap();

            let playerc = world.read::<Player>();
            let unitc = world.read::<Unit>();
            let posc = world.read::<Position>();

            match kind {
                EntityKind::Hero => {
                    let player = playerc.get(e).unwrap();
                    let pos = posc.get(e).unwrap().point;

                    events.push(Event::AddHero {
                        id: id,
                        hero: player.hero,
                        position: pos,
                        name: player.name().into(),
                    });
                }
                EntityKind::Projectile => {
                    let pos = posc.get(e).unwrap().point;
                    let unit = unitc.get(e).unwrap();

                    events.push(Event::AddProjectile {
                        id: id,
                        position: pos,
                        target: unit.target.clone(),
                    });
                }
            }
        }

        events
    }
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
    SetTarget(Target),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    EntityMove(EntityID, Point),
    AddHero {
        id: EntityID,
        position: Point,
        hero: Hero,
        name: String,
    },
    AddProjectile {
        id: EntityID,
        position: Point,
        target: Target,
    },
    RemoveEntity(EntityID),
}
