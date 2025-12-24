use asr::game_engine::unity::scene_manager;
use asr::game_engine::unity::scene_manager::{CppGameObject, Scene, SceneManager};
use asr::Process;
use helpers::error::SimpleError;
use helpers::watchers::{ValueGetter, Watcher};
use std::cell::Cell;
use std::error::Error;

fn get_scene_if_active(
    process: &Process,
    scene_manager: &SceneManager,
    scene: &str,
) -> Result<Scene, Box<dyn Error>> {
    let active_scene = scene_manager
        .get_current_scene(process)
        .map_err(|_| SimpleError::from("failed to get current scene"))?;

    let active_scene_name = active_scene
        .name(process, scene_manager)
        .map_err(|_| SimpleError::from("failed reading game object"))?;

    if scene != active_scene_name {
        return Err(SimpleError::from(&format!("unable to get game object path, in scene {active_scene_name} while expected scene was {}", scene)).into());
    }

    Ok(active_scene)
}

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
        let active_scene = get_scene_if_active(self.process, self.scene_manager, self.scene)
            .map_err(|e| {
                self.cached_object.set(None);
                e
            })?;

        // this is pretty jank, but we're using the cached address if one exists
        let game_object = match self.cached_object.take() {
            Some(game_object) => game_object,
            None => {
                let transform = active_scene
                    .find_transform(
                        self.process,
                        self.scene_manager,
                        self.root_object_name,
                        self.path,
                    )
                    .map_err(|_| SimpleError::from("couldnt find transform"))?;

                transform
                    .get_game_object(self.process, self.scene_manager)
                    .map_err(|_| SimpleError::from("couldnt get game_object"))?
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

// pub struct MonoBehaviourFieldPath<'a, T: CheckedBitPattern> {
//     _phantom: std::marker::PhantomData<T>,
//     process: &'a Process,
//     scene_manager: &'a scene_manager::SceneManager,
//
//     scene: &'static str,
//     root_object_name: &'static str,
//     game_object_path: &'static [&'static str],
//     component_name: &'static str,
//     field_path: &'static [&'static str],
//
//     cached_addr: Cell<Option<CppGameObject>>,
// }
