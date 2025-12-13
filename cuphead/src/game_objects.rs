use asr::string::ArrayCString;
use asr::{Address, PointerSize, Process};
use helpers::error::SimpleError;
use helpers::pointer::{PointerPath, PointerPathReadable, ValueReader};
use std::error::Error;

pub struct SceneManager<'a> {
    process: &'a Process,
    pub address: Address,
}

impl<'a> SceneManager<'a> {
    pub fn attach(process: &'a Process) -> Result<Self, Box<dyn Error>> {
        let module_address = process
            .get_module_address("Cuphead.exe")
            .map_err(|_| SimpleError::from("failed getting main module address"))?;

        // aga
        let address = module_address + 0x104FB78;

        Ok(Self { process, address })
    }

    pub fn active_scene(&self) -> Result<Scene<'a>, Box<dyn Error>> {
        Ok(Scene {
            process: self.process,
            path: PointerPath::new32(self.process, self.address, [0x0, 0x24]),
        })
    }

    // GameObjectPath::new("scene_cutscene_devil", "Cutscene", &["devil_cinematic_bad_ending_transition_0001"]);
    pub fn get_game_object_path(
        &self,
        scene: &str,
        root_object: &str,
        path: &[&str],
    ) -> Result<GameObject<'a>, Box<dyn Error>> {
        let active_scene = self.active_scene()?;
        let active_scene_name = active_scene.name()?;
        if scene != active_scene_name {
            return Err(SimpleError::from(&format!("unable to get game object path, in scene {active_scene_name} while expected scene was {scene}")).into());
        }

        let mut current_game_object = active_scene.find_root_object(root_object)?;
        for object_name in path {
            current_game_object = current_game_object.find_child(object_name)?;
        }

        Ok(current_game_object)
    }
}

pub struct Scene<'a> {
    process: &'a Process,
    pub path: PointerPath<'a, Process, u32>,
}

impl<'a> Scene<'a> {
    pub fn name(&self) -> Result<String, Box<dyn Error>> {
        let base: u32 = self.path.read()?;

        // if the string is small enough, it's just stored at 0x30, otherwise there's a pointer
        let string_ptr: u32 = self.path.child([0x0, 0x2C]).read()?;
        let string_ptr = if string_ptr != 0 {
            string_ptr
        } else {
            base + 0x30
        };

        let cstr = self
            .process
            .read::<ArrayCString<128>>(string_ptr)
            .map_err(|_| {
                SimpleError::from(&format!(
                    "failed reading string pointer at 0x{string_ptr:X}"
                ))
            })?;

        cstr.validate_utf8()
            .map(|c| c.to_owned())
            .map_err(|_| SimpleError::from("failed to parse unity scene name").into())
    }

    pub fn find_root_object(
        &self,
        root_object_name: &str,
    ) -> Result<GameObject<'a>, Box<dyn Error>> {
        let mut root_object_node = self.path.child::<u32>([0x0, 0x90]);
        loop {
            let current_object_node = root_object_node.child::<u32>([0x0, 0x8]);
            let object = GameObject {
                process: self.process,
                address: current_object_node.read()?.into(),
            };

            let name = object.name()?;
            if name == root_object_name {
                return Ok(object);
            }

            // next
            root_object_node = root_object_node.child([0x0, 0x4]);
        }
    }
}

pub struct GameObject<'a> {
    process: &'a Process,
    address: Address,
}

impl<'a> GameObject<'a> {
    pub fn name(&self) -> Result<String, Box<dyn Error>> {
        let name: PointerPath<_, ArrayCString<128>> =
            PointerPath::new32(self.process, self.address, [0x1C, 0x3C, 0x0]);
        let name = name.read()?;
        name.validate_utf8()
            .map(|c| c.to_owned())
            .map_err(|_| SimpleError::from("failed reading game object name").into())
    }

    pub fn find_child(&self, child_name: &str) -> Result<GameObject<'a>, Box<dyn Error>> {
        let children_count = self
            .process
            .read::<u32>(self.address + 0x58)
            .map_err(|_| SimpleError::from("failed reading game object children count"))?;
        let children: Address = self
            .process
            .read::<u32>(self.address + 0x50)
            .map_err(|_| SimpleError::from("failed reading game object children list"))?
            .into();

        for i in 0..children_count {
            let object = GameObject {
                process: self.process,
                address: self
                    .process
                    .read::<u32>(children + 0x4 * i as u64)
                    .map_err(|_| SimpleError::from("failed reading game object child"))?
                    .into(),
            };

            let name = object.name()?;

            if name == child_name {
                return Ok(object);
            }
        }

        Err(SimpleError::from(&format!("could not find object {child_name} in game tree")).into())
    }

    pub fn is_active_self(&self) -> Result<bool, Box<dyn Error>> {
        Ok(self
            .process
            .read_pointer_path::<bool>(self.address, PointerSize::Bit32, &[0x1C, 0x32])
            .map_err(|_| SimpleError::from("failed reading game object activeSelf"))?)
    }
}

pub struct GameObjectActivePath<'a> {
    process: &'a Process,
    scene_manager: &'a SceneManager<'a>,

    scene: &'static str,
    root_object_name: &'static str,
    path: &'static [&'static str],
}

impl<'a> GameObjectActivePath<'a> {
    pub fn new(
        process: &'a Process,
        scene_manager: &'a SceneManager<'a>,
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
        }
    }
}

impl<'a> ValueReader<'a, bool> for GameObjectActivePath<'a> {
    fn read(&self) -> Result<bool, Box<dyn Error>> {
        let game_object = self.scene_manager.get_game_object_path(
            self.scene,
            self.root_object_name,
            self.path,
        )?;

        game_object.is_active_self()
    }
}
