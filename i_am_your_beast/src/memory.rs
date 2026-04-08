use asr::game_engine::unity::scene_manager::SceneManager;
use asr::string::ArrayWString;
use helpers::watchers::unity::{MonoBehaviourFieldPath, UnityImage};
use helpers::watchers::Watcher;
use std::error::Error;
use std::rc::Rc;

pub struct Memory<'a> {
    // Level / scenes
    /// used to identify if we're on Mercy (25).
    /// note that this is only level number within a pack. which means ids <= 12 are ambiguous
    /// (multiple levels share that number)
    pub level: Watcher<'a, i32>,

    /// Level state
    pub level_state: Watcher<'a, i32>,

    /// Tracks if player is active. Makes split starts more precise.
    pub tracking: Watcher<'a, bool>,

    //#25 Mercy split
    pub cutscene_id: Watcher<'a, i32>,
    /// Technically a transition destination scene, but works as a current scene
    // Too lazy to make a watcher that caches the active scene or something
    // pub transition_scene: Watcher<'a, u64>,
    pub transition_scene: Watcher<'a, ArrayWString<128>>,
    pub scene_transition_state: Watcher<'a, i32>,

    // Level timers
    // Note that the following time values will stay whatever they last were while in the level select screen,
    //   and only reset once a level has been loaded.
    /// Total elapsed time since level start
    pub combat_time: Watcher<'a, f32>,
    /// Time "regained" from killing enemies, etc. (the IGT is combatTime - regainedCombatTime)
    pub regained_combat_time: Watcher<'a, f32>,
    /// The final time shown on the level complete screen
    pub ui_level_complete_time: Watcher<'a, f32>,
    /// Self explanatory. Stays true while in the level complete screen.
    pub timer_started: Watcher<'a, bool>,
}

impl<'a> Memory<'a> {
    pub fn new(
        unity: UnityImage<'a>,
        scene_manager: Rc<SceneManager>,
    ) -> Result<Memory<'a>, Box<dyn Error>> {
        Ok(Memory {
            level: Watcher::from(unity.path(
                "GameManager",
                0,
                &[
                    "instance",
                    "levelController",
                    "informationSetter",
                    "levelInformation",
                    "levelNumber",
                ],
            ))
            .default(),
            level_state: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "levelState"],
            ))
            .default(),
            tracking: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "gameplayTracker", "tracking"],
            ))
            .default(),
            cutscene_id: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "cutsceneInfoStorer", "sequence", "ID"],
            ))
            .default(),
            transition_scene: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "activeSceneTransition", "destination", "0x14"],
            )),
            scene_transition_state: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "activeSceneTransition", "state"],
            ))
            .default(),
            combat_time: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "combatTimer", "timer"],
            ))
            .default(),
            regained_combat_time: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "combatTimer", "regainedTime"],
            ))
            .default(),
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
            )?)
            .default(),
            timer_started: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "combatTimer", "timerStarted"],
            ))
            .default(),
        })
    }

    pub fn invalidate(&mut self) {
        self.level.invalidate();
        self.level_state.invalidate();
        self.tracking.invalidate();
        self.cutscene_id.invalidate();
        self.transition_scene.invalidate();
        self.scene_transition_state.invalidate();
        self.combat_time.invalidate();
        self.regained_combat_time.invalidate();
        self.ui_level_complete_time.invalidate();
        self.timer_started.invalidate();
    }
}
