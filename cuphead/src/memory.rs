use asr::string::ArrayWString;
use asr::Address64;
use helpers::pointer::{Invalidatable, MemoryWatcher, UnityImage, UnityPointerPath};
use std::error::Error;

pub struct Memory<'a> {
    pub done_loading: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub in_game: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub scene: MemoryWatcher<'a, UnityPointerPath<'a>, ArrayWString<128>>,
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
            scene: MemoryWatcher::from(unity.path(
                "SceneLoader",
                0,
                &[
                    "<SceneName>k__BackingField",
                    // 0x14 - offset into string contents
                    // (TODO - read this from an offsets object, and/or introduce some helper)
                    "0x14",
                ],
            )),

            in_game: MemoryWatcher::from(unity.path("PlayerData", 0, &["inGame"]))
                .default_given(false),
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
        self.in_game.invalidate();
        self.scene.invalidate();
        self.save_file_index.invalidate();
        self.save_files.invalidate();
    }

    pub fn is_loading(&self) -> Result<bool, Box<dyn Error>> {
        Ok(!self.done_loading.current()?)
    }
}
