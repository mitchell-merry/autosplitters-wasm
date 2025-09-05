use helpers::pointer::{Invalidatable, MemoryWatcher, UnityImage, UnityPointerPath};

pub struct Memory<'a> {
    pub done_loading: MemoryWatcher<'a, UnityPointerPath<'a>, bool>,
}

impl<'a> Memory<'a> {
    pub fn new(unity: UnityImage<'a>) -> Memory<'a> {
        let path = unity.path("SceneLoader", 0, &["_instance", "doneLoadingSceneAsync"]);

        Memory {
            done_loading: MemoryWatcher::from(path).default_given(true),
        }
    }

    pub fn invalidate(&mut self) {
        self.done_loading.invalidate();
    }
}