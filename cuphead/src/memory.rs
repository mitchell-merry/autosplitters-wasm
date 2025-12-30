use crate::enums::Grade;
use crate::enums::Levels;
use crate::enums::Mode;
use asr::game_engine::unity::scene_manager::SceneManager;
use asr::string::ArrayWString;
use asr::{Address64, PointerSize};
use helpers::watchers::unity::{GameObjectActivePath, MonoBehaviourFieldPath, UnityImage};
use helpers::watchers::Watcher;
use std::error::Error;
use std::rc::Rc;

pub struct Offsets {
    pub string_contents: &'static str,
}

impl Offsets {
    pub fn new(size: PointerSize) -> Offsets {
        match size {
            PointerSize::Bit64 => Offsets {
                string_contents: "0x14",
            },
            PointerSize::Bit32 => Offsets {
                string_contents: "0xC",
            },
            _ => Offsets {
                string_contents: "0x0", // n/a
            },
        }
    }
}

pub struct Memory<'a> {
    pub done_loading: Watcher<'a, bool>,
    pub insta: Watcher<'a, Address64>,
    pub scene: Watcher<'a, ArrayWString<128>>,
    pub in_game: Watcher<'a, bool>,
    pub level: Watcher<'a, Levels>,
    pub level_won: Watcher<'a, bool>,
    pub level_ending: Watcher<'a, bool>,
    pub level_time: Watcher<'a, f32>,
    pub level_grade: Watcher<'a, Grade>,
    pub level_difficulty: Watcher<'a, Mode>,
    pub lsd_time: Watcher<'a, f32>,
    pub kd_spaces_moved: Watcher<'a, i32>,
    pub level_is_dice: Watcher<'a, bool>,
    pub level_is_dice_main: Watcher<'a, bool>,
    pub devil_bad_ending_active: Watcher<'a, bool>,
    pub difficulty_ticker_started_counting: Watcher<'a, bool>,
    pub difficulty_ticker_finished_counting: Watcher<'a, bool>,
}

impl<'a> Memory<'a> {
    pub fn new(
        unity: UnityImage<'a>,
        scene_manager: Rc<SceneManager>,
    ) -> Result<Memory<'a>, Box<dyn Error>> {
        let offsets = Offsets::new(unity.module.get_pointer_size());
        Ok(Memory {
            done_loading: Watcher::from(unity.path(
                "SceneLoader",
                0,
                &["_instance", "doneLoadingSceneAsync"],
            ))
            .default_given(true),
            insta: Watcher::from(unity.path("SceneLoader", 0, &["_instance", "camera"])).default(),
            scene: Watcher::from(unity.path(
                "SceneLoader",
                0,
                &["<SceneName>k__BackingField", offsets.string_contents],
            ))
            .default(),

            in_game: Watcher::from(unity.path("PlayerData", 0, &["inGame"])).default_given(false),
            level: Watcher::from(unity.path("Level", 0, &["<PreviousLevel>k__BackingField"]))
                .default(),
            level_won: Watcher::from(unity.path("Level", 0, &["<Won>k__BackingField"]))
                .default_given(false),
            level_ending: Watcher::from(unity.path(
                "Level",
                0,
                &["<Current>k__BackingField", "<Ending>k__BackingField"],
            ))
            .default_given(false),
            level_time: Watcher::from(unity.path(
                "Level",
                0,
                &["<Current>k__BackingField", "<LevelTime>k__BackingField"],
            ))
            .default_given(0f32),
            level_grade: Watcher::from(unity.path("Level", 0, &["<Grade>k__BackingField"]))
                .default(),
            level_difficulty: Watcher::from(unity.path(
                "Level",
                0,
                &["<CurrentMode>k__BackingField"],
            ))
            .default(),
            lsd_time: Watcher::from(unity.path(
                "Level",
                0,
                &["<ScoringData>k__BackingField", "time"],
            ))
            .default_given(0f32),
            kd_spaces_moved: Watcher::from(unity.path(
                "DicePalaceMainLevelGameInfo",
                0,
                &["PLAYER_SPACES_MOVED"],
            ))
            .default(),
            level_is_dice: Watcher::from(unity.path(
                "Level",
                0,
                &["<IsDicePalace>k__BackingField"],
            ))
            .default(),
            level_is_dice_main: Watcher::from(unity.path(
                "Level",
                0,
                &["<IsDicePalaceMain>k__BackingField"],
            ))
            .default(),
            devil_bad_ending_active: Watcher::from(GameObjectActivePath::new(
                unity.process,
                scene_manager.clone(),
                "scene_cutscene_devil",
                "Cutscene",
                &["devil_cinematic_bad_ending_transition_0001"],
            ))
            .default(),
            difficulty_ticker_started_counting: Watcher::from(MonoBehaviourFieldPath::init(
                unity.process,
                unity.module.clone(),
                scene_manager.clone(),
                "scene_win",
                "WinScreen",
                &["UI", "Canvas", "Scoring", "DifficultyTicker"],
                "WinScreenTicker",
                &["startedCounting"],
            )?)
            .default(),
            difficulty_ticker_finished_counting: Watcher::from(MonoBehaviourFieldPath::init(
                unity.process,
                unity.module.clone(),
                scene_manager.clone(),
                "scene_win",
                "WinScreen",
                &["UI", "Canvas", "Scoring", "DifficultyTicker"],
                "WinScreenTicker",
                &["<FinishedCounting>k__BackingField"],
            )?)
            .default(),
        })
    }

    pub fn invalidate(&mut self) {
        self.done_loading.invalidate();
        self.insta.invalidate();
        self.scene.invalidate();
        self.in_game.invalidate();
        self.level.invalidate();
        self.level_won.invalidate();
        self.level_ending.invalidate();
        self.level_time.invalidate();
        self.level_grade.invalidate();
        self.level_difficulty.invalidate();
        self.lsd_time.invalidate();
        self.kd_spaces_moved.invalidate();
        self.level_is_dice.invalidate();
        self.level_is_dice_main.invalidate();
        self.devil_bad_ending_active.invalidate();
        self.difficulty_ticker_started_counting.invalidate();
        self.difficulty_ticker_finished_counting.invalidate();
    }

    pub fn is_loading(&self) -> Result<bool, Box<dyn Error>> {
        Ok(!self.done_loading.current()?)
    }
}
