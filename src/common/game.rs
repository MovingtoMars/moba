use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use na::{self, Point2};
use ncollide::query::PointQuery;
use specs;

use common::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Target {
    Nothing,
    Position(Point),
    Entity(EntityID),
}

pub struct Game {
    entity_ids: Vec<EntityID>,
    players: Vec<EntityID>,
    next_entity_id: Arc<Mutex<u32>>,
    entity_map: Arc<Mutex<HashMap<EntityID, specs::Entity>>>,
    world: specs::World,
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
        w.register::<BasicAttacker>();

        Game {
            entity_ids: Vec::new(),
            players: Vec::new(),
            next_entity_id: Arc::new(Mutex::new(0)),
            entity_map: Arc::new(Mutex::new(HashMap::new())),
            world: w,
        }
    }

    pub fn players(&self) -> &[EntityID] {
        &self.players
    }

    pub fn next_entity_id(&mut self) -> EntityID {
        let mut next_entity_id = self.next_entity_id.lock().unwrap();
        let t = *next_entity_id;
        *next_entity_id = next_entity_id.checked_add(1).unwrap();
        EntityID(t)
    }

    pub fn add_player(
        &mut self,
        id: EntityID,
        hero: logic::HeroKind,
        name: String,
        position: Point,
        team: Option<Team>,
    ) -> EntityID {
        let e = self.add_entity(id, EntityKind::Hero, |entity| {
            let mut e = entity
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
                .with(BasicAttacker {
                    attack_speed: hero.attack_speed(),
                    time_until_next_attack: 0.0,
                })
                .with(Hitpoints::new_at_max(50))
                .with(Velocity::new(0.0, 0.0));

            if let Some(team) = team {
                e = e.with(team);
            }

            e
        });

        self.players.push(e);
        e
    }

    pub fn add_projectile(
        &mut self,
        id: EntityID,
        position: Point,
        target: Target,
        damage: u16,
        team: Option<Team>,
        owner: EntityID,
    ) -> EntityID {
        self.add_entity(id, EntityKind::Projectile, |entity| {
            let mut e = entity
                .with(Position { point: position })
                .with(Projectile { damage, owner })
                .with(Renderable {
                    radius: 5.0,
                    colour: [1.0, 0.0, 0.0, 1.0],
                })
                .with(Hitbox::new_ball(5.0))
                .with(Unit {
                    speed: 800.0,
                    target: target,
                })
                .with(Velocity::new(0.0, 0.0));

            if let Some(team) = team {
                e = e.with(team);
            }

            e
        })
    }

    pub fn add_entity<F>(&mut self, id: EntityID, kind: EntityKind, f: F) -> EntityID
    where
        F: FnOnce(specs::EntityBuilder) -> specs::EntityBuilder,
    {
        let entity = self.world.create_entity();
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

    pub fn mut_world(&mut self) -> &mut specs::World {
        &mut self.world
    }

    pub fn with_component<T: specs::Component, U, F>(&mut self, e: EntityID, f: F) -> Option<U>
    where
        F: FnOnce(&T) -> U,
    {
        let entity = match self.get_entity(e) {
            Some(entity) => entity,
            None => return None,
        };

        let storage = self.world.read::<T>();
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

        let mut storage = self.world.write::<T>();
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

        let hb_storage = self.world.read::<Hitbox>();
        let hb = match hb_storage.get(entity) {
            Some(x) => x,
            None => return false,
        };
        let pos_storage = self.world.read::<Position>();
        let pos = match pos_storage.get(entity) {
            Some(x) => x,
            None => return false,
        };

        let hb_pos = na::Isometry2::new(pos.point.into(), na::zero());

        hb.shape_handle.contains_point(&hb_pos, &Point2::new(x, y)) // XXX
    }

    pub fn run_command(&mut self, command: Command, origin: EntityID) -> Vec<Event> {
        let entity = self.get_entity(origin).unwrap();
        let mut events = Vec::new();

        match command {
            Command::SetTarget(target) => {
                let mut tc = self.world.write::<Unit>();

                tc.get_mut(entity).unwrap().target = match target {
                    Target::Entity(id) if id == origin => Target::Nothing, // XXX error?
                    x => x,
                };
            }

            Command::UseAbility {
                ability_id,
                mouse_position,
            } => {
                let eid = self.next_entity_id();
                let positionc = self.world.read::<Position>();
                let teamc = self.world.read::<Team>();
                let p = positionc.get(entity).unwrap().point;
                let e = Event::AddProjectile {
                    id: eid,
                    position: p,
                    target: Target::Position(mouse_position.unwrap()),
                    damage: 10,
                    owner: origin,
                    team: teamc.get(entity).cloned(),
                };
                events.push(e);
            }
        }

        events
    }

    pub fn remove_entity(&mut self, id: EntityID) {
        let e = self.get_entity(id).unwrap();
        self.entity_map.lock().unwrap().remove(&id);
        self.players.retain(|&p| p != id);
        self.entity_ids.retain(|&x| x != id);
        self.world.delete_entity(e);
    }

    pub fn run_event(&mut self, event: Event) {
        println!("{:?}", event);
        match event {
            Event::RemoveEntity(id) => {
                self.remove_entity(id);
            }
            Event::EntityMove(id, point) => {
                let e = self.get_entity(id).unwrap();
                let mut posc = self.world.write::<Position>();
                posc.get_mut(e).unwrap().point = point;
            }
            Event::AddHero {
                id,
                hero,
                position,
                name,
                team,
            } => {
                self.add_player(id, hero, name, position, team);
            }
            Event::AddProjectile {
                id,
                position,
                target,
                damage,
                owner,
                team,
            } => {
                self.add_projectile(id, position, target, damage, team, owner);
            }
            Event::DamageEntity { id, damage } => {
                if let Some(e) = self.get_entity(id) {
                    let mut hitpointsc = self.world.write::<Hitpoints>();
                    hitpointsc.get_mut(e).map(|x| x.damage(damage)); // XXX
                }
            }
        }
    }

    pub fn run_events(&mut self, events: &[Event]) {
        for e in events {
            self.run_event(e.clone());
        }
    }

    pub fn tick(&mut self, time: f64) -> Vec<Event> {
        self.world.maintain();

        let context = Context::new(time, self.entity_map.clone(), self.next_entity_id.clone());
        self.world.add_resource(context.clone()); // XXX
        let mut dispatcher = register_systems(specs::DispatcherBuilder::new()).build();
        dispatcher.dispatch(&mut self.world.res);

        let events = context.events();

        self.run_events(&events);

        self.world.maintain();

        events
    }

    pub fn events_for_loading(&mut self) -> Vec<Event> {
        let mut events = Vec::new();

        for &id in &self.entity_ids {
            let e = self.get_entity(id).unwrap();
            let world = &mut self.world;
            let kind = *world.read::<EntityKind>().get(e).unwrap();

            let playerc = world.read::<Player>();
            let unitc = world.read::<Unit>();
            let posc = world.read::<Position>();
            let teamc = world.read::<Team>();
            let projectilec = world.read::<Projectile>();

            match kind {
                EntityKind::Hero => {
                    let player = playerc.get(e).unwrap();
                    let pos = posc.get(e).unwrap().point;

                    events.push(Event::AddHero {
                        id: id,
                        hero: player.hero,
                        position: pos,
                        name: player.name().into(),
                        team: teamc.get(e).cloned(),
                    });
                }
                EntityKind::Projectile => {
                    let pos = posc.get(e).unwrap().point;
                    let unit = unitc.get(e).unwrap();
                    let proj = projectilec.get(e).unwrap();
                    let team = teamc.get(e).cloned();

                    events.push(Event::AddProjectile {
                        id,
                        position: pos,
                        target: unit.target.clone(),
                        damage: proj.damage,
                        team,
                        owner: proj.owner,
                    });
                }
            }
        }

        events
    }
}
