use std::collections::{VecDeque, HashMap};
use std::ops::{Sub, Deref, DerefMut};
use na::{Point2, Vector2};
use ecs;

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
            Hero::John => 100.0,
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

#[derive(Clone, Debug)]
pub struct Velocity {
    raw: Point,
}

impl Velocity {
    pub fn new(x: f64, y: f64) -> Self {
        Velocity { raw: Point::new(x, y) }
    }
}

// TODO: proper conditional impl of Clone depending on component types.
components! {
    #[derive(Clone)]
    struct MyComponents {
        #[hot] id: EntityID,
        #[hot] kind: EntityKind,
        #[hot] position: Point,
        #[cold] player: Player,
        #[hot] renderable: Renderable,
        #[hot] velocity: Velocity,
    }
}

pub struct UpdateVelocityProcess;

impl ecs::System for UpdateVelocityProcess {
    type Components = MyComponents;
    type Services = ();
}

impl ecs::system::EntityProcess for UpdateVelocityProcess {
    fn process(&mut self,
               entities: ecs::EntityIter<MyComponents>,
               data: &mut ecs::DataHelper<MyComponents, ()>) {

    }
}

systems! {
    struct MySystems<MyComponents, ()> {
        active: {
            motion: ecs::system::EntitySystem<UpdateVelocityProcess> = ecs::system::EntitySystem::new(
                UpdateVelocityProcess,
                aspect!(<MyComponents> all: [position, velocity])
            ),
        },
        passive: {}
    }
}

pub struct Game {
    entity_ids: Vec<EntityID>,
    players: Vec<EntityID>,
    next_entity_id: u32,
    world: ecs::World<MySystems>,
    entity_map: HashMap<EntityID, ecs::Entity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    Hero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityID(u32);

impl Game {
    pub fn new() -> Self {
        Game {
            entity_ids: Vec::new(),
            players: Vec::new(),
            next_entity_id: 0,
            world: ecs::World::new(),
            entity_map: HashMap::new(),
        }
    }

    // pub fn clone_into(&self, other: Game) -> Game {
    //     Game {
    //         entity_ids: self.entity_ids.clone(),
    //         entity_map: self.entity_map.clone(),
    //         next_entity_id: self.next_entity_id,
    //         world: ecs::World {
    //             systems: other.world.systems,
    //             data: self.world.data.clone(),
    //         },
    //         players: self.players.clone(),
    //     }
    // }

    pub fn players(&self) -> &[EntityID] {
        &self.players
    }

    pub fn next_entity_id(&mut self) -> EntityID {
        let t = self.next_entity_id;
        self.next_entity_id = self.next_entity_id.checked_add(1).unwrap();
        EntityID(t)
    }

    pub fn add_player(&mut self, hero: Hero, name: String, position: Point) -> EntityID {
        let e = self.add_entity(EntityKind::Hero,
                                |entity: ecs::BuildData<MyComponents>, data: &mut MyComponents| {
            data.position.add(&entity, position);
            data.player.add(&entity,
                            Player {
                                hero: hero,
                                name: name,
                                target: Target::Nothing,
                            });
            data.renderable.add(&entity,
                                Renderable {
                                    radius: hero.radius(),
                                    colour: [0.0, 1.0, 0.0, 1.0],
                                });
            data.velocity.add(&entity, Velocity::new(0.0, 0.0));
        });

        self.players.push(e);
        e
    }

    pub fn add_entity<F>(&mut self, kind: EntityKind, f: F) -> EntityID
        where F: FnOnce(ecs::BuildData<MyComponents>, &mut MyComponents)
    {
        let id = self.next_entity_id();

        let entity = self.world
            .create_entity(|entity: ecs::BuildData<MyComponents>, data: &mut MyComponents| {
                // data.position.add(&entity, Position { x: 0.0, y: 0.0 });
                // data.velocity.add(&entity, Velocity { dx: 1.0, dy: 0.0 });
                data.id.add(&entity, id);
                data.kind.add(&entity, kind);
                f(entity, data)
            });

        self.entity_map.insert(id, entity);
        self.entity_ids.push(id);

        id
    }

    pub fn get_entity(&self, id: EntityID) -> Option<ecs::Entity> {
        self.entity_map.get(&id).cloned()
    }

    pub fn entity_ids(&self) -> &[EntityID] {
        &self.entity_ids
    }

    pub fn with_entity_data<F, R>(&mut self, id: EntityID, f: F) -> Option<R>
        where F: FnMut(ecs::EntityData<MyComponents>, &mut MyComponents) -> R
    {
        self.world.with_entity_data(self.entity_map.get(&id).unwrap(), f)
    }

    pub fn run_command(&mut self, command: Command, origin: EntityID) {
        let mut entity = self.get_entity(origin).unwrap();

        self.world.modify_entity(entity, |entity: ecs::ModifyData<MyComponents>,
                                  data: &mut MyComponents| {
            match command {
                Command::Move(target) => data.player[entity].target = Target::Position(target),
            }
        });
    }

    pub fn tick(&mut self, time: f64) {}
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
