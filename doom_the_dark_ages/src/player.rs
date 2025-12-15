use crate::physics::IdVec3;
use asr::Process;
use helpers::watchers::pointer_path::PointerPath;
use idtech::IdTech;
use std::error::Error;

pub struct IdPlayer<'a> {
    pub velocity: IdVec3<'a>,
}

impl<'a> IdPlayer<'a> {
    pub fn init(
        idtech: &IdTech<'a>,
        path: PointerPath<'a, Process>,
    ) -> Result<Self, Box<dyn Error>> {
        let player_c = idtech.get_class("Game", "idPlayer")?;
        let player_physics_c = idtech.get_class("Game", "idPlayerPhysicsInfo")?;
        let player_state_c = idtech.get_class("Game", "playerPState_t")?;

        Ok(IdPlayer {
            velocity: IdVec3::init(
                idtech,
                path.child([
                    player_c.get_offset("idPlayerPhysicsInfo")?,
                    player_physics_c.get_offset("current")?
                        + player_state_c.get_offset("velocity")?,
                ]),
            )?,
        })
    }

    pub fn invalidate(&mut self) {
        self.velocity.invalidate();
    }
}
