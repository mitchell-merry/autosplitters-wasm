use crate::enums::Levels;
use asr::string::ArrayWString;
use asr::PointerSize;
use helpers::pointer::{Invalidatable, MemoryWatcher, UnityImage, UnityPointerPath};
use std::error::Error;

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
                string_contents: "0x0",
            }, // n/a
        }
    }
}

pub struct Memory<'a> {
    pub done_loading: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub scene: MemoryWatcher<'a, UnityPointerPath<'a>, ArrayWString<128>>,
    pub in_game: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub level: MemoryWatcher<'a, UnityPointerPath<'a>, Levels>,
    pub level_won: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub level_ending: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub level_time: MemoryWatcher<'a, UnityPointerPath<'a>, f32>,
    pub lsd_time: MemoryWatcher<'a, UnityPointerPath<'a>, f32>,
    pub kd_spaces_moved: MemoryWatcher<'a, UnityPointerPath<'a>, i32>,
    pub level_is_dice: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub level_is_dice_main: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
}

impl<'a> Memory<'a> {
    pub fn new(unity: UnityImage<'a>) -> Memory<'a> {
        let offsets = Offsets::new(unity.module.pointer_size);
        Memory {
            done_loading: MemoryWatcher::from(unity.path(
                "SceneLoader",
                0,
                &["_instance", "doneLoadingSceneAsync"],
            ))
            .default_given(true),
            scene: MemoryWatcher::from(unity.path(
                "SceneLoader",
                0,
                &["<SceneName>k__BackingField", offsets.string_contents],
            ))
            .default(),

            in_game: MemoryWatcher::from(unity.path("PlayerData", 0, &["inGame"]))
                .default_given(false),
            level: MemoryWatcher::from(unity.path("Level", 0, &["<PreviousLevel>k__BackingField"]))
                .default(),
            level_won: MemoryWatcher::from(unity.path("Level", 0, &["<Won>k__BackingField"]))
                .default_given(false),
            level_ending: MemoryWatcher::from(unity.path(
                "Level",
                0,
                &["<Current>k__BackingField", "<Ending>k__BackingField"],
            ))
            .default_given(false),
            level_time: MemoryWatcher::from(unity.path(
                "Level",
                0,
                &["<Current>k__BackingField", "<LevelTime>k__BackingField"],
            ))
            .default_given(0f32),
            lsd_time: MemoryWatcher::from(unity.path(
                "Level",
                0,
                &["<ScoringData>k__BackingField", "time"],
            ))
            .default_given(0f32),
            kd_spaces_moved: MemoryWatcher::from(unity.path(
                "DicePalaceMainLevelGameInfo",
                0,
                &["PLAYER_SPACES_MOVED"],
            ))
            .default(),
            level_is_dice: MemoryWatcher::from(unity.path(
                "Level",
                0,
                &["<IsDicePalace>k__BackingField"],
            ))
            .default(),
            level_is_dice_main: MemoryWatcher::from(unity.path(
                "Level",
                0,
                &["<IsDicePalaceMain>k__BackingField"],
            ))
            .default(),
        }
    }

    pub fn invalidate(&mut self) {
        self.done_loading.invalidate();
        self.scene.invalidate();
        self.in_game.invalidate();
        self.level.invalidate();
        self.level_won.invalidate();
        self.level_ending.invalidate();
        self.level_time.invalidate();
        self.lsd_time.invalidate();
        self.kd_spaces_moved.invalidate();
        self.level_is_dice.invalidate();
        self.level_is_dice_main.invalidate();
    }

    pub fn is_loading(&self) -> Result<bool, Box<dyn Error>> {
        Ok(!self.done_loading.current()?)
    }
}
