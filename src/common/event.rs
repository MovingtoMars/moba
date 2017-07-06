use common::*;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    EntityMove(EntityID, Point),
    AddHero {
        id: EntityID,
        position: Point,
        hero: Hero,
        name: String,
        team: Option<Team>,
    },
    AddProjectile {
        id: EntityID,
        position: Point,
        target: Target,
        damage: u16,
    },
    DamageEntity { id: EntityID, damage: u16 },
    RemoveEntity(EntityID),
}
