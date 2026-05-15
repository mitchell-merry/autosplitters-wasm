use asr::game_engine::unity::scene_manager::SceneManager;
use helpers::watchers::unity::UnityImage;
use helpers::watchers::Watcher;
use std::error::Error;
use std::rc::Rc;

pub struct Memory<'a> {
    pub filename: Watcher<'a, u32>,
}

impl<'a> Memory<'a> {
    pub fn new(
        unity: UnityImage<'a>,
        scene_manager: Rc<SceneManager>,
    ) -> Result<Memory<'a>, Box<dyn Error>> {
        Ok(Memory {
            filename: Watcher::from(unity.path(
                "GlobalData",
                1,
                &["_instance", "_gameData", "fileName"],
            ))
            .default(),
        })
    }

    pub fn invalidate(&mut self) {
        // self.global_timer.invalidate();
        // self.timer.invalidate();
        // self.game_manager.invalidate();
        self.filename.invalidate();
    }
}
