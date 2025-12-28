use asr::game_engine::unity::scene_manager::SceneManager;
use asr::Address64;
use helpers::watchers::unity::UnityImage;
use helpers::watchers::Watcher;
use std::error::Error;
use std::rc::Rc;

pub struct Memory<'a> {
    pub global_timer: Watcher<'a, Address64>,
}

impl<'a> Memory<'a> {
    pub fn new(
        unity: UnityImage<'a>,
        scene_manager: Rc<SceneManager>,
    ) -> Result<Memory<'a>, Box<dyn Error>> {
        Ok(Memory {
            global_timer: Watcher::from(unity.path("GlobalTimer", 2, &["Instance"])),
        })
    }

    pub fn invalidate(&mut self) {}
}
