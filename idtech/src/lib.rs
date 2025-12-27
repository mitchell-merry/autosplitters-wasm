mod typeinfo;

use std::error::Error;
use std::rc::Rc;

use crate::typeinfo::class::ClassTypeInfo;
use asr::{signature::Signature, Address, Process};
use helpers::error::SimpleError;
use typeinfo::*;

pub struct IdTech<'a> {
    _process: &'a Process,
    _memory: Rc<Memory>,
    type_info: Rc<TypeInfoTools<'a>>,
}

impl<'a> IdTech<'a> {
    pub async fn try_load(
        process: &'a Process,
        version: IdTechVersion,
        main_module_name: &str,
    ) -> Result<IdTech<'a>, Box<dyn Error>> {
        asr::print_message(&format!("  => idtech: Using version {version:?}"));

        let memory = Rc::new(Memory::new(process, version, main_module_name)?);
        let typeinfo_instance = process
            .read::<u64>(memory.typeinfo_addr)
            .map_err(|_| SimpleError::from("failed to read typeinfo_addr ()"))?
            .into();
        asr::print_message(&format!(
            "  => found typeinfo instance at 0x{typeinfo_instance:?}"
        ));

        if typeinfo_instance == Address::NULL {
            return Err(SimpleError::from("idtech: the typeinfo instance is null").into());
        }

        let type_info = Rc::new(TypeInfoTools::try_load(process, typeinfo_instance)?);

        let idtech = IdTech {
            _process: process,
            _memory: memory,
            type_info,
        };

        Ok(idtech)
    }

    pub fn get_offset(
        &self,
        project_name: &str,
        class_name: &str,
        variable_name: &str,
    ) -> Result<u64, Box<dyn Error>> {
        let class = self.get_class(project_name, class_name)?;
        Ok(*class.get_variable(variable_name)?.get_offset()?)
    }

    pub fn get_class(
        &self,
        project_name: &str,
        class_name: &str,
    ) -> Result<&ClassTypeInfo<'a>, Box<dyn Error>> {
        let project = self
            .type_info
            .get_project(project_name)
            .ok_or(SimpleError::from(&format!(
                "failed to find project {project_name}"
            )))?;

        let class = project
            .get_class(class_name)
            .ok_or(SimpleError::from(&format!(
                "failed to find class {class_name} in project {project_name}"
            )))?;

        Ok(class)
    }

    // pub fn invalidate_cache(&mut self) -> Result<(), Error> {
    //     Ok(())
    // }
}

// disclaimer: I don't know much about the different idtech versions work...
// i have only tried this with a few games
#[derive(Clone, Copy, Debug)]
pub enum IdTechVersion {
    IdTech8, // Doom: The Dark Ages
}

type ScanFn =
    fn(process: &Process, module_range: (Address, u64)) -> Result<Address, Box<dyn Error>>;

fn find_addr_or_panic(
    name: &str,
    process: &Process,
    module_range: (Address, u64),
    sigs: Vec<ScanFn>,
) -> Address {
    for (i, sig) in sigs.iter().enumerate() {
        if let Ok(addr) = sig(process, module_range) {
            asr::print_message(&format!(
                "  => Found {name} at 0x{addr} with signature index {i}"
            ));
            return addr;
        }
    }

    panic!("unable to find addr for {name}");
}

fn scan<const N: usize>(
    signature: Signature<N>,
    process: &Process,
    (addr, len): (Address, u64),
    offset: u32,
    next_instruction: u32,
) -> Result<Address, Box<dyn Error>> {
    let addr = signature
        .scan_process_range(process, (addr, len))
        .ok_or(SimpleError::from("unable to find signature in memory"))?
        + offset;

    Ok(addr
        + process
            .read::<u32>(addr)
            .map_err(|_| SimpleError::from(&format!("unable to read from address 0x{}", addr)))?
        + next_instruction)
}

pub struct Memory {
    typeinfo_addr: Address,

    _offsets: Offsets,
}

impl Memory {
    fn new(
        process: &Process,
        version: IdTechVersion,
        main_module_name: &str,
    ) -> Result<Memory, Box<dyn Error>> {
        let module_range = process
            .get_module_range(main_module_name)
            .map_err(|_| SimpleError::from("failed to get module range of main module"))?;

        let typeinfo_sigs: Vec<ScanFn> = vec![|p, mr| {
            scan(
                    Signature::<29>::new(
                        "48 8b fa 4c 89 41 08 48 8b d9 48 85 D2 74 25 48 8B 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? 89 03",
                    ),
                    p,
                    mr,
                    0x12,
                    0x4,
                )
        }];

        Ok(Memory {
            typeinfo_addr: find_addr_or_panic("typeinfo", process, module_range, typeinfo_sigs),
            _offsets: Offsets::new(version),
        })
    }
}

struct Offsets {}

impl Offsets {
    fn new(version: IdTechVersion) -> Self {
        match version {
            IdTechVersion::IdTech8 => Self {},
        }
    }
}
