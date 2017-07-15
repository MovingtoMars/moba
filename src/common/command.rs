use common::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command {
    SetTarget(Target),
    UseAbility {
        ability_id: u32,
        mouse_position: Option<Point>,
    },
}
