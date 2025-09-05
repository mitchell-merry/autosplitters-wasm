use asr::string::ArrayCString;
use helpers::pointer::{Invalidatable, MemoryWatcher, UnityImage, UnityPointerPath};

pub struct Memory<'a> {
    pub done_loading: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub in_game: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
    pub scene: MemoryWatcher<'a, UnityPointerPath<'a>, ArrayCString<128>>,
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
            in_game: MemoryWatcher::from(unity.path("PlayerData", 0, &["inGame"]))
                .default_given(true),
            scene: MemoryWatcher::from(unity.path(
                "SceneLoader",
                0,
                &["<SceneName>k__BackingField"],
            )),
        }
    }

    pub fn invalidate(&mut self) {
        self.done_loading.invalidate();
        self.in_game.invalidate();
    }
}
