use common::*;

use specs;

// pub trait Ability {
//     fn run(&self, &Game) -> Vec<Event>;
// }

// pub trait Hero {
//     fn kind() -> HeroKind;
//     fn ability_zero(&self) -> Box<Ability>;
// }
//
// pub struct John {}
//
// impl Hero for John {
//     fn kind() -> HeroKind {
//         HeroKind::John
//     }
//
//     fn ability_zero(&self) -> Box<Ability> {
//         Box::new(ShootAbility)
//     }
// }
//
// pub struct ShootAbility;
//
// impl Ability for ShootAbility {
//     fn run(&self, game: &Game) -> Vec<Event> {
//         Vec::new()
//     }
// }

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub enum HeroKind {
    John,
}

impl HeroKind {
    pub fn radius(self) -> f64 {
        match self {
            HeroKind::John => 50.0,
        }
    }

    pub fn speed(self) -> f64 {
        match self {
            HeroKind::John => 200.0,
        }
    }

    pub fn range(self) -> f64 {
        match self {
            HeroKind::John => 200.0,
        }
    }

    pub fn attack_speed(self) -> f64 {
        match self {
            HeroKind::John => 0.8,
        }
    }
}

pub fn can_attack(
    this: specs::Entity,
    other: specs::Entity,
    teamc: &RS<Team>,
    hitpointsc: &RS<Hitpoints>,
) -> bool {
    let team1 = teamc.get(this);
    let team2 = teamc.get(other);
    if team1 == team2 && team1 != None {
        return false;
    }

    if hitpointsc.get(other).is_none() {
        return false;
    }

    true
}
