use std::collections::{VecDeque, HashMap};
use std::ops::{Sub, Deref, DerefMut};
use std::sync::{Arc, Mutex};
use na::{Point2, Vector2};
use specs::{self, Join};

#[derive(Debug, Clone)]
pub struct Position {
    pub point: Point,
}

impl specs::Component for Position {
    type Storage = specs::VecStorage<Position>;
}

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
            Hero::John => 200.0,
        }
    }
}


#[derive(Clone, Debug)]
pub enum Target {
    Nothing,
    Position(Point),
}


#[derive(Clone, Debug)]
pub struct Player {
    hero: Hero,
    name: String,
    target: Target,
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
    speed: f64,
    target: Target,
}

impl specs::Component for Unit {
    type Storage = specs::VecStorage<Unit>;
}

#[derive(Clone, Debug)]
pub struct Velocity {
    x: f64,
    y: f64,
}

impl specs::Component for Velocity {
    type Storage = specs::VecStorage<Velocity>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    Hero,
}

impl specs::Component for EntityKind {
    type Storage = specs::VecStorage<EntityKind>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityID(u32);

impl specs::Component for EntityID {
    type Storage = specs::VecStorage<EntityID>;
}

pub struct UpdateVelocitySystem;

impl specs::System<Context> for UpdateVelocitySystem {
    fn run(&mut self, arg: specs::RunArg, c: Context) {
        let (unitc, mut velocityc, positionc, idc, playerc) = arg.fetch(|w| {
            (w.read::<Unit>(),
             w.write::<Velocity>(),
             w.read::<Position>(),
             w.read::<EntityID>(),
             w.read::<Player>())
        });

        for (unit, velocity, player, position) in
            (&unitc, &mut velocityc, &playerc, &positionc).iter() {
            let mut speed = unit.speed;

            *velocity = match unit.target {
                Target::Nothing => Velocity { x: 0.0, y: 0.0 },
                Target::Position(p) => {
                    let dx = p.x - position.point.x;
                    let dy = p.y - position.point.y;
                    let d = (dx * dx + dy * dy).sqrt();
                    if speed * c.time > d {
                        speed = d;
                    }
                    let mut ratio = speed / d;

                    Velocity {
                        x: ratio * dx,
                        y: ratio * dy,
                    }
                }
            };
        }
    }
}

pub struct MotionSystem;

impl specs::System<Context> for MotionSystem {
    fn run(&mut self, arg: specs::RunArg, c: Context) {
        let (idc, velocityc, mut positionc) =
            arg.fetch(|w| (w.read::<EntityID>(), w.read::<Velocity>(), w.write::<Position>()));

        for (&id, velocity, mut position) in (&idc, &velocityc, &mut positionc).iter() {
            let x = position.point.x + velocity.x * c.time;
            let y = position.point.y + velocity.y * c.time;

            let event = Event::EntityMove(id, Point::new(x, y));
            c.push_event(event);
        }
    }
}

pub struct ContextInner {
    events: Vec<Event>,
}

#[derive(Clone)]
pub struct Context {
    time: f64,
    inner: Arc<Mutex<ContextInner>>,
}

impl Context {
    fn new(time: f64) -> Self {
        Context {
            time: time,
            inner: Arc::new(Mutex::new(ContextInner { events: Vec::new() })),
        }
    }

    fn push_event(&self, event: Event) {
        self.inner.lock().unwrap().events.push(event);
    }
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

    pub fn add_player(&mut self, hero: Hero, name: String, position: Point) -> EntityID {
        let e = self.add_entity(EntityKind::Hero, |entity| {
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

    pub fn add_entity<F>(&mut self, kind: EntityKind, f: F) -> EntityID
        where F: FnOnce(specs::EntityBuilder<()>) -> specs::EntityBuilder<()>
    {
        let id = self.next_entity_id();

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
        match event {
            Event::EntityMove(id, point) => {
                let e = self.get_entity(id).unwrap();
                self.run_custom(move |arg| {
                    let mut posc = arg.fetch(|w| w.write::<Position>());
                    posc.get_mut(e).unwrap().point = point;
                });
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

        let events = context.inner.lock().unwrap().events.clone();

        self.run_events(&events);
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
}
