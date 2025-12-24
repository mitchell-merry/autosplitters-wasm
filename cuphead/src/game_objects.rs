use asr::game_engine::unity::scene_manager;
use asr::game_engine::unity::scene_manager::CppGameObject;
use asr::Process;
use helpers::error::SimpleError;
use helpers::watchers::{ValueGetter, Watcher};
use std::cell::Cell;
use std::error::Error;

pub struct GameObjectActivePath<'a> {
    process: &'a Process,
    scene_manager: &'a scene_manager::SceneManager,

    scene: &'static str,
    root_object_name: &'static str,
    path: &'static [&'static str],

    cached_object: Cell<Option<CppGameObject>>,
}

impl<'a> GameObjectActivePath<'a> {
    pub fn new(
        process: &'a Process,
        scene_manager: &'a scene_manager::SceneManager,
        scene: &'static str,
        root_object_name: &'static str,
        path: &'static [&'static str],
    ) -> Self {
        GameObjectActivePath {
            process,
            scene_manager,
            scene,
            root_object_name,
            path,
            cached_object: Cell::new(None),
        }
    }
}

impl<'a> ValueGetter<bool> for GameObjectActivePath<'a> {
    fn get(&self) -> Result<bool, Box<dyn Error>> {
        let active_scene = self
            .scene_manager
            .get_current_scene(self.process)
            .map_err(|_| SimpleError::from("failed to get current scene"))?;

        let active_scene_name = active_scene
            .name(self.process, self.scene_manager)
            .map_err(|_| SimpleError::from("failed reading game object"))?;

        if self.scene != active_scene_name {
            self.cached_object.set(None);

            return Err(SimpleError::from(&format!("unable to get game object path, in scene {active_scene_name} while expected scene was {}", self.scene)).into());
        }

        // this is pretty jank, but we're using the cached address if one exists
        let game_object = match self.cached_object.take() {
            Some(game_object) => game_object,
            None => {
                let mut current_transform = self
                    .scene_manager
                    .get_root_game_object(self.process, self.root_object_name)
                    .map_err(|_| SimpleError::from("couldnt find root object"))?;

                for object_name in self.path {
                    current_transform = current_transform
                        .get_child(self.process, self.scene_manager, object_name)
                        .map_err(|_| SimpleError::from("could not find the child"))?;
                }

                let game_object = current_transform
                    .get_game_object(self.process, self.scene_manager)
                    .map_err(|_| SimpleError::from("couldnt get game_object"))?;

                game_object
            }
        };

        self.cached_object.set(Some(game_object.clone()));

        game_object
            .is_active_in_hierarchy(self.process, self.scene_manager)
            .map_err(|_| SimpleError::from("couldnt get is active").into())
    }
}

impl<'a> From<GameObjectActivePath<'a>> for Watcher<'a, bool> {
    fn from(value: GameObjectActivePath<'a>) -> Self {
        Watcher::new(Box::new(value))
    }
}
