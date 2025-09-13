use crate::enums::Levels;
use asr::string::ArrayWString;
use asr::Address64;
use helpers::pointer::{Invalidatable, MemoryWatcher, UnityImage, UnityPointerPath};
use std::error::Error;

pub struct Memory<'a> {
    pub done_loading: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub scene_loader_instance: MemoryWatcher<'a, UnityPointerPath<'a>, Address64>,
    pub currently_loading: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub scene: MemoryWatcher<'a, UnityPointerPath<'a>, ArrayWString<128>>,
    pub previous_scene: MemoryWatcher<'a, UnityPointerPath<'a>, ArrayWString<128>>,
    pub in_game: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub level: MemoryWatcher<'a, UnityPointerPath<'a>, Levels>,
    pub level_won: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub level_ending: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub level_time: MemoryWatcher<'a, UnityPointerPath<'a>, f32>,
    pub lsd_time: MemoryWatcher<'a, UnityPointerPath<'a>, f32>,
    pub kd_spaces_moved: MemoryWatcher<'a, UnityPointerPath<'a>, i32>,
    pub level_is_dice: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub level_is_dice_main: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub save_file_index: MemoryWatcher<'a, UnityPointerPath<'a>, u32>,
    pub save_files: MemoryWatcher<'a, UnityPointerPath<'a>, Address64>,
}

impl<'a> Memory<'a> {
    pub fn new(unity: UnityImage<'a>) -> Memory<'a> {
        Memory {
            done_loading: MemoryWatcher::from(unity.path(
                "SceneLoader",
                0,
                &["_instance", "doneLoadingSceneAsync"],
            ))
            .default_given(true),
            scene_loader_instance: MemoryWatcher::from(unity.path(
                "SceneLoader",
                0,
                &["_instance"],
            ))
            .default_given(0x0.into()),
            currently_loading: MemoryWatcher::from(unity.path(
                "SceneLoader",
                0,
                &["_instance", "currentlyLoading"],
            ))
            .default_given(true),
            scene: MemoryWatcher::from(unity.path(
                "SceneLoader",
                0,
                &[
                    "<SceneName>k__BackingField",
                    // 0x14 - offset into string contents
                    // (TODO - read this from an offsets object, and/or introduce some helper)
                    "0xC",
                ],
            ))
            .default(),
            previous_scene: MemoryWatcher::from(unity.path(
                "SceneLoader",
                0,
                &[
                    "previousSceneName",
                    // 0x14 - offset into string contents
                    // (TODO - read this from an offsets object, and/or introduce some helper)
                    "0xC",
                ],
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
            save_file_index: MemoryWatcher::from(unity.path(
                "PlayerData",
                0,
                &["_CurrentSaveFileIndex"],
            )),
            save_files: MemoryWatcher::from(unity.path("PlayerData", 0, &["_saveFiles"])),
        }
    }

    pub fn invalidate(&mut self) {
        self.done_loading.invalidate();
        self.scene_loader_instance.invalidate();
        self.currently_loading.invalidate();
        self.scene.invalidate();
        self.previous_scene.invalidate();
        self.in_game.invalidate();
        self.level.invalidate();
        self.level_won.invalidate();
        self.level_ending.invalidate();
        self.level_time.invalidate();
        self.lsd_time.invalidate();
        self.kd_spaces_moved.invalidate();
        self.level_is_dice.invalidate();
        self.level_is_dice_main.invalidate();
        self.save_file_index.invalidate();
        self.save_files.invalidate();
    }

    pub fn is_loading(&self) -> Result<bool, Box<dyn Error>> {
        Ok(!self.done_loading.current()?)
    }
}
