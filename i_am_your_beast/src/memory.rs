use asr::game_engine::unity::scene_manager::SceneManager;
use helpers::watchers::unity::{MonoBehaviourFieldPath, UnityImage};
use helpers::watchers::Watcher;
use std::error::Error;
use std::rc::Rc;

pub struct Memory<'a> {
    pub combat_time: Watcher<'a, f32>,
    pub ui_level_complete_time: Watcher<'a, f32>,
}

impl<'a> Memory<'a> {
    pub fn new(
        unity: UnityImage<'a>,
        scene_manager: Rc<SceneManager>,
    ) -> Result<Memory<'a>, Box<dyn Error>> {
        Ok(Memory {
            combat_time: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "combatTimer", "timer"],
            )),
            ui_level_complete_time: Watcher::from(MonoBehaviourFieldPath::init(
                unity.process,
                unity.module.clone(),
                scene_manager.clone(),
                "#5_Corridor_StartingGun",
                "[LEVEL DEPENDENCIES]",
                &[
                    "Level Controller",
                    "Level Complete Canvas(Clone)",
                    "Canvas",
                    "Scale Anchor",
                    "Main Anchor",
                    "Time Score",
                ],
                "UILevelCompleteTimeScoreBar",
                &["currentTime"],
            )?),
        })
    }

    pub fn invalidate(&mut self) {
        self.combat_time.invalidate();
        self.ui_level_complete_time.invalidate();
    }
}
