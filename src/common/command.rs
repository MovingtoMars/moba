use common::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command {
    SetTarget(Target),
}
