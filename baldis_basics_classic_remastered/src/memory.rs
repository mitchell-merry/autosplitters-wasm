use asr::game_engine::unity::scene_manager::SceneManager;
use asr::PointerSize;
use helpers::watchers::unity::{ActiveSceneNameGetter, UnityImage};
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
    pub velocity: Watcher<'a, f32>,
    pub x: Watcher<'a, u64>,
    pub xx: Watcher<'a, u64>,
    pub scene: Watcher<'a, String>,
    pub ready_to_start: Watcher<'a, bool>,
}

impl<'a> Memory<'a> {
    pub fn new(
        unity: UnityImage<'a>,
        scene_manager: Rc<SceneManager>,
    ) -> Result<Memory<'a>, Box<dyn Error>> {
        let offsets = Offsets::new(unity.module.get_pointer_size());
        Ok(Memory {
            velocity: Watcher::from(unity.path(
                "CoreGameManager",
                1,
                &["m_Instance", "players", "0x20", "plm", "frameVelocity"],
            ))
            .default_given(0f32),
            x: Watcher::from(unity.path("CoreGameManager", 1, &["m_Instance"])),
            xx: Watcher::from(unity.path(
                "CoreGameManager",
                1,
                &["m_Instance", "players", "0x20", "plm"],
            )),
            scene: Watcher::from(ActiveSceneNameGetter::new(
                unity.process,
                scene_manager.clone(),
            )),
            ready_to_start: Watcher::from(unity.path(
                "CoreGameManager",
                1,
                &["m_Instance", "readyToStart"],
            )),
        })
    }

    pub fn invalidate(&mut self) {
        self.velocity.invalidate();
        self.x.invalidate();
        self.xx.invalidate();
        self.scene.invalidate();
        self.ready_to_start.invalidate();
    }
}
