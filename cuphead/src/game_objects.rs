use asr::signature::Signature;
use asr::string::ArrayCString;
use asr::{print_message, Address, Address32, PointerSize, Process};
use helpers::error::SimpleError;
use helpers::memory::scan_rel;
use helpers::watchers::pointer_path::{PointerPath, PointerPathReadable};
use helpers::watchers::{ValueGetter, Watcher};
use std::cell::Cell;
use std::error::Error;

pub struct SceneManager<'a> {
    process: &'a Process,
    pub address: Address,
}

impl<'a> SceneManager<'a> {
    pub fn attach(process: &'a Process) -> Result<Self, Box<dyn Error>> {
        // 1.0,1.1.5,1.2.4 windows: 55 8B EC E8 ?? ?? ?? ?? 8B C8 E8 ?? ?? ?? ?? 85 C0, 0x4
        const SIG: Signature<17> =
            Signature::new("55 8B EC E8 ?? ?? ?? ?? 8B C8 E8 ?? ?? ?? ?? 85 C0");

        let main_module = process
            .get_name()
            .map_err(|_| SimpleError::from("cant get process name"))?;
        print_message(&format!("module name {:?}", main_module));

        let addr = scan_rel(&SIG, process, &main_module, 0x4, 0x4)?;
        print_message(&format!("scan_rel {:?}", addr));

        let real_addr = addr
            + process
                .read::<i32>(addr + 0x2)
                .map_err(|_| SimpleError::from("can't read"))?
            + 0x4;

        // aga
        // let module_address = process
        //     .get_main_module_range()
        //     .map_err(|_| SimpleError::from("failed getting main module address"))?;
        // let address = module_address.0 + 0x104FB78;

        Ok(Self {
            process,
            address: real_addr,
        })
    }

    pub fn active_scene(&self) -> Result<Scene<'a>, Box<dyn Error>> {
        Ok(Scene {
            process: self.process,
            path: PointerPath::new(self.process, self.address, PointerSize::Bit32, [0x0, 0x24]),
        })
    }
}

pub struct Scene<'a> {
    process: &'a Process,
    pub path: PointerPath<'a, Process>,
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
        let mut root_object_node = self.path.child([0x0, 0x90]);
        loop {
            let current_object_node = root_object_node.child([0x0, 0x8]);
            let object = GameObject {
                process: self.process,
                address: current_object_node.read::<Address32>()?.into(),
            };

            let name = object.name()?;
            if name == root_object_name {
                return Ok(object);
            }

            // next
            root_object_node = root_object_node.child([0x0, 0x4]);
        }
    }

    pub fn find_game_object_in_scene(
        &self,
        root_object_name: &str,
        path: &'static [&'static str],
    ) -> Result<GameObject<'a>, Box<dyn Error>> {
        let mut current_game_object = self.find_root_object(root_object_name)?;
        for object_name in path {
            current_game_object = current_game_object.find_child(object_name)?;
        }

        Ok(current_game_object)
    }
}

pub struct GameObject<'a> {
    process: &'a Process,
    address: Address,
}

impl<'a> GameObject<'a> {
    pub fn name(&self) -> Result<String, Box<dyn Error>> {
        let name = PointerPath::new(
            self.process,
            self.address,
            PointerSize::Bit32,
            [0x1C, 0x3C, 0x0],
        );
        let name: ArrayCString<128> = name.read()?;
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

    cached_object: Cell<Option<GameObject<'a>>>,
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
            cached_object: Cell::new(None),
        }
    }
}

impl<'a> ValueGetter<bool> for GameObjectActivePath<'a> {
    fn get(&self) -> Result<bool, Box<dyn Error>> {
        let active_scene = self.scene_manager.active_scene()?;
        let active_scene_name = active_scene.name()?;
        if self.scene != active_scene_name {
            self.cached_object.set(None);

            return Err(SimpleError::from(&format!("unable to get game object path, in scene {active_scene_name} while expected scene was {}", self.scene)).into());
        }

        // this is pretty jank, but we're using the cached address if one exists
        let game_object = match self.cached_object.take() {
            Some(game_object) => game_object,
            None => active_scene.find_game_object_in_scene(self.root_object_name, self.path)?,
        };

        let active = game_object.is_active_self()?;

        self.cached_object.set(Some(game_object));

        Ok(active)
    }
}

impl<'a> From<GameObjectActivePath<'a>> for Watcher<'a, bool> {
    fn from(value: GameObjectActivePath<'a>) -> Self {
        Watcher::new(Box::new(value))
    }
}
