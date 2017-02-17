use std::collections::{VecDeque, HashMap};
use std::ops::{Sub, Deref, DerefMut};
use std::sync::{Arc, Mutex};
use na::{Point2, Vector2};
use specs::{self, Join};

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


#[derive(Clone, Debug)]
pub enum Target {
    Nothing,
    Position(Point),
}

pub struct Game {
    entity_ids: Vec<EntityID>,
    players: Vec<EntityID>,
    next_entity_id: u32,
    entity_map: HashMap<EntityID, specs::Entity>,
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

        let mut planner = specs::Planner::new(w, 4);

        planner.add_system(UpdateVelocitySystem, "UpdateVelocitySystem", 100);
        planner.add_system(MotionSystem, "MotionSystem", 99);

        Game {
            entity_ids: Vec::new(),
            players: Vec::new(),
            next_entity_id: 0,
            entity_map: HashMap::new(),
            planner: planner,
        }
    }

    pub fn players(&self) -> &[EntityID] {
        &self.players
    }

    pub fn next_entity_id(&mut self) -> EntityID {
        let t = self.next_entity_id;
        self.next_entity_id = self.next_entity_id.checked_add(1).unwrap();
        EntityID(t)
    }

    pub fn add_player(&mut self,
                      id: EntityID,
                      hero: Hero,
                      name: String,
                      position: Point)
                      -> EntityID {
        let e = self.add_entity(id, EntityKind::Hero, |entity| {
            entity.with(Position { point: position })
                .with(Player {
                    hero: hero,
                    name: name,
                    target: Target::Nothing,
                })
                .with(Renderable {
                    radius: hero.radius(),
                    colour: [0.0, 1.0, 0.0, 1.0],
                })
                .with(Unit {
                    speed: hero.speed(),
                    target: Target::Nothing,
                })
                .with(Velocity { x: 0.0, y: 0.0 })
        });

        self.players.push(e);
        e
    }

    pub fn add_entity<F>(&mut self, id: EntityID, kind: EntityKind, f: F) -> EntityID
        where F: FnOnce(specs::EntityBuilder<()>) -> specs::EntityBuilder<()>
    {
        let mut entity = self.planner.mut_world().create_now();
        let entity = entity.with(kind).with(id);
        let entity = f(entity);
        let entity = entity.build();

        self.entity_map.insert(id, entity);
        self.entity_ids.push(id);

        id
    }

    pub fn get_entity(&self, id: EntityID) -> Option<specs::Entity> {
        self.entity_map.get(&id).cloned()
    }

    pub fn entity_ids(&self) -> &[EntityID] {
        &self.entity_ids
    }

    pub fn run_custom<F>(&mut self, f: F)
        where F: 'static + Send + FnOnce(specs::RunArg)
    {
        self.planner.run_custom(f)
    }

    pub fn mut_world(&mut self) -> &mut specs::World {
        self.planner.mut_world()
    }

    pub fn with_component<T: specs::Component, U, F>(&mut self, e: EntityID, f: F) -> Option<U>
        where F: FnOnce(&T) -> U
    {
        let entity = match self.entity_map.get(&e) {
            Some(&entity) => entity,
            None => return None,
        };

        let mut world = self.planner.mut_world();
        let mut storage = world.read::<T>();
        let component = match storage.get(entity) {
            Some(x) => x,
            None => return None,
        };

        Some(f(component))
    }

    pub fn clone_component<T: specs::Component + Clone>(&mut self, e: EntityID) -> Option<T> {
        self.with_component::<T, _, _>(e, |c| c.clone())
    }

    pub fn run_command(&mut self, command: Command, origin: EntityID) {
        let mut entity = self.get_entity(origin).unwrap();

        // self.world.modify_entity(entity, |entity: ecs::ModifyData<MyComponents>,
        //                           data: &mut MyComponents| {
        //     match command {
        //         Command::Move(target) => data.player[entity].target = Target::Position(target),
        //     }
        // });

        match command {
            Command::Move(target) => {
                self.run_custom(move |arg| {
                    let mut t = arg.fetch(|world| world.write::<Unit>());
                    t.get_mut(entity).unwrap().target = Target::Position(target)
                });
            }
        }
    }

    pub fn run_event(&mut self, event: Event) {
        println!("{:?}", event);
        match event {
            Event::EntityMove(id, point) => {
                let e = self.get_entity(id).unwrap();
                self.run_custom(move |arg| {
                    let mut posc = arg.fetch(|w| w.write::<Position>());
                    posc.get_mut(e).unwrap().point = point;
                });
            }
            Event::AddHero { id, hero, position, name } => {
                self.add_player(id, hero, name, position);
            }
        }
    }

    pub fn run_events(&mut self, events: &[Event]) {
        for e in events {
            self.run_event(e.clone());
        }
    }

    pub fn tick(&mut self, time: f64) -> Vec<Event> {
        let context = Context::new(time);
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
    Move(Point),
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
}
