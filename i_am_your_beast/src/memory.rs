use crate::enums::{LevelState, SceneTransitionState};
use asr::game_engine::unity::scene_manager::SceneManager;
use helpers::watchers::unity::{
    ActiveSceneNameGetter, GameObjectActivePath, MonoBehaviourFieldPath, StringMatch, UnityImage,
};
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
    pub level_state: Watcher<'a, LevelState>,

    /// Tracks if player is active. Makes split starts more precise.
    pub tracking: Watcher<'a, bool>,

    //#25 Mercy split
    pub cutscene_id: Watcher<'a, i32>,
    pub scene: Watcher<'a, String>,
    pub scene_transition_state: Watcher<'a, SceneTransitionState>,

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
    /// Whether the credits are active
    pub credits_active: Watcher<'a, bool>,
    /// The page in the credits we're at
    pub credits_index: Watcher<'a, u32>,
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
            .default_on_fail(),
            level_state: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "levelState"],
            ))
            .default_on_fail(),
            tracking: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "gameplayTracker", "tracking"],
            ))
            .default_on_fail(),
            cutscene_id: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "cutsceneInfoStorer", "sequence", "ID"],
            ))
            .default_on_fail(),
            scene: Watcher::from(ActiveSceneNameGetter::new(
                unity.process,
                scene_manager.clone(),
            ))
            .use_old_on_fail(),
            scene_transition_state: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "activeSceneTransition", "state"],
            ))
            .default_on_fail(),
            combat_time: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "combatTimer", "timer"],
            ))
            .default_on_fail(),
            regained_combat_time: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "combatTimer", "regainedTime"],
            ))
            .default_on_fail(),
            ui_level_complete_time: Watcher::from(MonoBehaviourFieldPath::init(
                unity.process,
                unity.module.clone(),
                scene_manager.clone(),
                StringMatch::Any,
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
            // .default_on_fail(),
            timer_started: Watcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "combatTimer", "timerStarted"],
            ))
            .default_on_fail(),
            credits_active: Watcher::from(GameObjectActivePath::new(
                unity.process,
                scene_manager.clone(),
                StringMatch::Exact("Start Screen"),
                "Credits UI",
                &[],
            ))
            .default_on_fail(),
            credits_index: Watcher::from(MonoBehaviourFieldPath::init(
                unity.process,
                unity.module.clone(),
                scene_manager.clone(),
                StringMatch::Exact("Start Screen"),
                "Credits UI",
                &[],
                "UICreditsRoot",
                &["totalIndex"],
            )?)
            .default_on_fail(),
        })
    }

    pub fn invalidate(&mut self) {
        self.level.invalidate();
        self.level_state.invalidate();
        self.tracking.invalidate();
        self.cutscene_id.invalidate();
        self.scene.invalidate();
        self.scene_transition_state.invalidate();
        self.combat_time.invalidate();
        self.regained_combat_time.invalidate();
        self.ui_level_complete_time.invalidate();
        self.timer_started.invalidate();
        self.credits_active.invalidate();
        self.credits_index.invalidate();
    }
}
