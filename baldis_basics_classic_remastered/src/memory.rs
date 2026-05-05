use asr::game_engine::unity::scene_manager::SceneManager;
use asr::string::ArrayWString;
use asr::PointerSize;
use helpers::watchers::unity::{
    ActiveSceneNameGetter, GameObjectActivePath, StringMatch, UnityImage,
};
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
    pub transition_active: Watcher<'a, bool>,
    pub win_screen_active: Watcher<'a, bool>,
    pub current_level: Watcher<'a, u32>,
    pub current_level_title: Watcher<'a, ArrayWString<128>>,
    pub next_level_title: Watcher<'a, ArrayWString<128>>,
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
            ))
            .default_given(true),
            transition_active: Watcher::from(unity.path(
                "GlobalCam",
                1,
                &["m_Instance", "transitionActive"],
            ))
            .default_given(false),
            win_screen_active: Watcher::from(
                GameObjectActivePath::new(
                    unity.process,
                    scene_manager.clone(),
                    StringMatch::Exact("DontDestroyOnLoad"),
                    "ClassicWin(Clone)",
                    &[],
                )
                .cache_object(false),
            )
            .default_given(false),
            current_level: Watcher::from(unity.path(
                "CoreGameManager",
                1,
                &["m_Instance", "sceneObject", "levelNo"],
            )), // .default_given(false),
            current_level_title: Watcher::from(unity.path(
                "CoreGameManager",
                1,
                &[
                    "m_Instance",
                    "sceneObject",
                    "levelTitle",
                    offsets.string_contents,
                ],
            )), // .default_given(false),
            next_level_title: Watcher::from(unity.path(
                "CoreGameManager",
                1,
                &[
                    "m_Instance",
                    "sceneObject",
                    "nextLevel",
                    "levelTitle",
                    offsets.string_contents,
                ],
            )), // .default_given(false),
        })
    }

    pub fn invalidate(&mut self) {
        self.velocity.invalidate();
        self.x.invalidate();
        self.xx.invalidate();
        self.scene.invalidate();
        self.ready_to_start.invalidate();
        self.transition_active.invalidate();
        self.win_screen_active.invalidate();
        self.current_level.invalidate();
        self.current_level_title.invalidate();
        self.next_level_title.invalidate();
    }
}
