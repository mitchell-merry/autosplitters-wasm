use asr::game_engine::unity::mono::{Class, Module};
use asr::game_engine::unity::scene_manager::{CppGameObject, Scene, SceneManager};
use asr::{Address, Process};
use bytemuck::CheckedBitPattern;
use helpers::error::SimpleError;
use helpers::watchers::{ValueGetter, Watcher};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::marker::PhantomData;
use std::rc::Rc;

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
        .map_err(|_| SimpleError::from("failed getting active scene name"))?;

    if scene != active_scene_name {
        return Err(SimpleError::from(&format!("unable to get game object path, in scene {active_scene_name} while expected scene was {}", scene)).into());
    }

    Ok(active_scene)
}

pub struct GameObjectActivePath<'a> {
    process: &'a Process,
    scene_manager: Rc<SceneManager>,

    scene: &'static str,
    root_object_name: &'static str,
    path: &'static [&'static str],

    cached_object: Cell<Option<CppGameObject>>,
}

impl<'a> GameObjectActivePath<'a> {
    pub fn new(
        process: &'a Process,
        scene_manager: Rc<SceneManager>,
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
        let active_scene = get_scene_if_active(self.process, &self.scene_manager, self.scene)
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
                        &self.scene_manager,
                        self.root_object_name,
                        self.path,
                    )
                    .map_err(|_| SimpleError::from("couldnt find transform"))?;

                transform
                    .get_game_object(self.process, &self.scene_manager)
                    .map_err(|_| SimpleError::from("couldnt get game_object"))?
            }
        };

        self.cached_object.set(Some(game_object.clone()));

        game_object
            .is_active_in_hierarchy(self.process, &self.scene_manager)
            .map_err(|_| SimpleError::from("couldnt get is active").into())
    }
}

impl<'a> From<GameObjectActivePath<'a>> for Watcher<'a, bool> {
    fn from(value: GameObjectActivePath<'a>) -> Self {
        Watcher::new(Box::new(value))
    }
}

struct MBFPInternal {
    offsets: [u64; 128],
    resolved_offsets: usize,
    depth: usize,
}

pub struct MonoBehaviourFieldPath<'a, T: CheckedBitPattern> {
    _phantom: PhantomData<T>,
    process: &'a Process,
    module: Rc<Module>,
    scene_manager: Rc<SceneManager>,

    scene: &'static str,
    root_object_name: &'static str,
    game_object_path: &'static [&'static str],
    component_type_name: &'static str,
    field_path: &'static [&'static str],

    inner: RefCell<MBFPInternal>,

    cached_component: Cell<Option<Address>>,
}

impl<'a, T: CheckedBitPattern> MonoBehaviourFieldPath<'a, T> {
    pub fn init(
        process: &'a Process,
        module: Rc<Module>,
        scene_manager: Rc<SceneManager>,
        scene: &'static str,
        root_object_name: &'static str,
        game_object_path: &'static [&'static str],
        component_type_name: &'static str,
        field_path: &'static [&'static str],
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            _phantom: PhantomData,
            process,
            module,
            scene_manager,
            scene,
            root_object_name,
            game_object_path,
            component_type_name,
            field_path,
            inner: RefCell::new(MBFPInternal {
                resolved_offsets: 0,
                offsets: [0; 128],
                depth: field_path.len(),
            }),
            cached_component: Cell::new(None),
        })
    }
}

// FIXME: all of this is very jank
impl<'a, T: CheckedBitPattern> ValueGetter<T> for MonoBehaviourFieldPath<'a, T> {
    fn get(&self) -> Result<T, Box<dyn Error>> {
        let active_scene = get_scene_if_active(self.process, &self.scene_manager, self.scene)
            .inspect_err(|_| self.cached_component.set(None))?;

        // this is pretty jank, but we're using the cached address if one exists
        let mut current_object = match self.cached_component.take() {
            Some(component) => component,
            None => {
                let transform = active_scene
                    .find_transform(
                        self.process,
                        &self.scene_manager,
                        self.root_object_name,
                        self.game_object_path,
                    )
                    .map_err(|_| SimpleError::from("couldnt find transform"))?;

                transform
                    .get_game_object(self.process, &self.scene_manager)
                    .map_err(|_| SimpleError::from("couldnt get game_object"))?
                    .get_class(self.process, &self.scene_manager, self.component_type_name)
                    .map_err(|_| SimpleError::from("couldnt find component in game object"))?
            }
        };

        // starts as the component
        self.cached_component.set(Some(current_object));
        let component = current_object;

        let mut inner = self.inner.borrow_mut();
        for i in 0..inner.resolved_offsets {
            current_object = self
                .process
                .read_pointer(
                    current_object + inner.offsets[i],
                    self.module.get_pointer_size(),
                )
                .map_err(|_| {
                    SimpleError::from("couldnt dereference with already resolved offset")
                })?;
        }

        for i in inner.resolved_offsets..inner.depth {
            let current_class = Class::from_object(self.process, &self.module, current_object)
                .map_err(|_| SimpleError::from("couldnt get class from object"))?;

            let offset = current_class
                .get_field_offset(self.process, &self.module, self.field_path[i])
                .ok_or(SimpleError::from("couldnt get field from class"))?;

            inner.offsets[i] = offset as _;
            inner.resolved_offsets += 1;

            current_object = self
                .process
                .read_pointer(current_object + offset, self.module.get_pointer_size())
                .map_err(|_| SimpleError::from("couldnt dereference with retrieved offset"))?;
        }

        let p = &inner.offsets[..inner.depth];

        self.process
            .read_pointer_path::<T>(component, self.module.get_pointer_size(), p)
            .map_err(|_| {
                SimpleError::from(&format!("couldnt read final bit {}, {:X?}", component, p)).into()
            })
    }
}

impl<'a, T: CheckedBitPattern + 'a> From<MonoBehaviourFieldPath<'a, T>> for Watcher<'a, T> {
    fn from(value: MonoBehaviourFieldPath<'a, T>) -> Self {
        Watcher::new(Box::new(value))
    }
}
