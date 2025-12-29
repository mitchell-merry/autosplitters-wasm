use asr::game_engine::unity::scene_manager::SceneManager;
use helpers::watchers::unity::UnityImage;
use helpers::watchers::Watcher;
use std::error::Error;
use std::rc::Rc;

pub struct Memory<'a> {
    // pub global_timer: Watcher<'a, Address64>,
    // pub timer: Watcher<'a, Address64>,
    // pub game_manager: Watcher<'a, Address64>,
    pub time: Watcher<'a, u64>,
    pub started_timestamp: Watcher<'a, u64>,
    pub running: Watcher<'a, bool>,
}

impl<'a> Memory<'a> {
    pub fn new(
        unity: UnityImage<'a>,
        scene_manager: Rc<SceneManager>,
    ) -> Result<Memory<'a>, Box<dyn Error>> {
        Ok(Memory {
            // global_timer: Watcher::from(unity.path("GlobalTimer", 2, &["Instance"])),
            // timer: Watcher::from(unity.path("Timer", 1, &["Instance"])),
            // game_manager: Watcher::from(unity.path("GameManager", 2, &["Instance"])),
            time: Watcher::from(unity.path(
                "Timer",
                1,
                &["Instance", "_currentRunTimer", "elapsed"],
            ))
            .default(),
            started_timestamp: Watcher::from(unity.path(
                "Timer",
                1,
                &["Instance", "_currentRunTimer", "started"],
            ))
            .default(),
            running: Watcher::from(unity.path(
                "Timer",
                1,
                &["Instance", "_currentRunTimer", "is_running"],
            ))
            .default(),
        })
    }

    pub fn invalidate(&mut self) {
        // self.global_timer.invalidate();
        // self.timer.invalidate();
        // self.game_manager.invalidate();
        self.time.invalidate();
        self.started_timestamp.invalidate();
        self.running.invalidate();
    }
}
