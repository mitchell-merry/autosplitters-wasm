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

        print_message(&format!("Attaching module address: {}", address));

        Ok(Self { process, address })
    }

    pub fn active_scene(&self) -> Result<Scene, Box<dyn Error>> {
        Ok(Scene {
            process: self.process,
            path: PointerPath::new32(self.process, self.address, [0x0, 0x24]),
        })
    }
}

pub struct Scene<'a> {
    process: &'a Process,
    path: PointerPath<'a, Process>,
}

impl<'a> Scene<'a> {
    pub fn name(&self) -> Result<String, Box<dyn Error>> {
        let base: u32 = self.path.read()?;

        let string_ptr: u32 = self.path.child([0x0, 0x2C]).read()?;
        let string_ptr = if string_ptr != 0 {
            string_ptr
        } else {
            base + 0x30
        };

        print_message(&format!("name: {:X}", string_ptr));

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
