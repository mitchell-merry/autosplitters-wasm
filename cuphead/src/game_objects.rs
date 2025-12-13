use asr::string::ArrayCString;
use asr::{print_message, Address, Process};
use helpers::error::SimpleError;
use helpers::pointer::{PointerPath, Readable2};
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
    ) -> Result<PointerPath<'a, Process>, Box<dyn Error>> {
        let active_scene = self.active_scene()?;
        let active_scene_name = active_scene.name()?;
        print_message(&format!("in scene {active_scene_name}"));
        if scene != active_scene_name {
            return Err(SimpleError::from(&format!("unable to get game object path, in scene {active_scene_name} while expected scene was {scene}")).into());
        }

        // find root object
        let mut root_object_node = active_scene.path.child([0x0, 0x90]);
        loop {
            let current_object_node = root_object_node.child([0x0, 0x8]);
            let name = current_object_node.child([0x0, 0x1C, 0x3C, 0x0]);
            let name = name.read::<ArrayCString<128>>()?;
            let name = name
                .validate_utf8()
                .map_err(|_| SimpleError::from("failed reading root game object name"))?;

            if name == root_object {
                root_object_node = current_object_node;
                break;
            }

            // next
            root_object_node = root_object_node.child([0x0, 0x4]);
        }

        let mut current_game_object = root_object_node;
        for object in path {
            let children_count = current_game_object.child([0x0, 0x58]).read::<u32>()?;
            let children = current_game_object.child([0x0, 0x50]);

            let mut i = 0;
            current_game_object = loop {
                if i >= children_count {
                    return Err(SimpleError::from(&format!(
                        "could not find object {object} in game tree"
                    ))
                    .into());
                }

                let child = children.child([0x0, 0x4 * i as u64]);

                let name = child
                    .child([0x0, 0x1C, 0x3C, 0x0])
                    .read::<ArrayCString<128>>()?;
                let name = name
                    .validate_utf8()
                    .map_err(|_| SimpleError::from("failed reading game object name"))?;

                if name == *object {
                    break child;
                }

                i += 1;
            }
        }

        Ok(current_game_object)
    }
}

pub struct GameObjectPath {}

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
}

struct GameObject {
    address: Address,
}
