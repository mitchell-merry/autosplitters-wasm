// Please don't go after me idSoftware
// I like you :)
use crate::typeinfo::class::ClassTypeInfo;
use asr::string::ArrayCString;
use asr::{Address, Process};
use helpers::error::SimpleError;
use std::collections::HashMap;
use std::error::Error;
pub mod class;

// idArray < idTypeInfoTools::registeredTypeInfo_t , 2 > generatedTypeInfo; // 0x00000 (size: 0x70) -
const TYPE_INFO_TOOLS_GENERATED_TYPE_INFO_OFFSET: u64 = 0x0;

const TYPE_INFO_PROJECT_SIZE: u64 = 0x38;
// typeInfoGenerated_t typeInfoGenerated;  // 0x00000 (size: 0x8) - type info generated with the TypeInfoGen
const TYPE_INFO_PROJECT_TYPE_INFO_GENERATED_OFFSET: u64 = 0x0;

// char projectName; // 0x00000 (size: 0x8) -
const TYPE_INFO_PROJECT_NAME_OFFSET: u64 = 0x0;
// classTypeInfo_t* classes
const TYPE_INFO_PROJECT_CLASSES_OFFSET: u64 = 0x18;
// int numClasses
const TYPE_INFO_PROJECT_NUM_CLASSES_OFFSET: u64 = 0x20;

const CLASS_TYPE_INFO_SIZE: u64 = 0x58;

/** Mirrors the idTypeInfoTools class */
pub struct TypeInfoTools<'a> {
    _process: &'a Process,
    _address: Address,

    projects: HashMap<String, TypeInfoProject<'a>>,
}

impl<'a> TypeInfoTools<'a> {
    pub fn try_load(
        process: &'a Process,
        address: Address,
    ) -> Result<TypeInfoTools<'a>, Box<dyn Error>> {
        let mut projects = HashMap::new();

        let projects_base = address + TYPE_INFO_TOOLS_GENERATED_TYPE_INFO_OFFSET;
        // TODO: should the range be dynamic?
        for i in 0..2 {
            let current_project = projects_base + i * TYPE_INFO_PROJECT_SIZE;
            let project = TypeInfoProject::init(process, current_project)?;
            projects.insert(project.name.clone(), project);
        }

        Ok(TypeInfoTools {
            _process: process,
            _address: address,
            projects,
        })
    }

    pub fn get_project(&self, project_name: &str) -> Option<&TypeInfoProject<'a>> {
        self.projects.get(project_name)
    }
}

/** Mirrors the idTypeInfoTools::registeredTypeInfo_t class, which has a projectName */
pub struct TypeInfoProject<'a> {
    _process: &'a Process,
    _address: Address,

    name: String,
    classes: HashMap<String, ClassTypeInfo<'a>>,
}

impl<'a> TypeInfoProject<'a> {
    pub fn init(
        process: &'a Process,
        address: Address,
    ) -> Result<TypeInfoProject<'a>, Box<dyn Error>> {
        let name = process
            .read_pointer_path::<ArrayCString<512>>(
                address,
                asr::PointerSize::Bit64,
                &[
                    TYPE_INFO_PROJECT_TYPE_INFO_GENERATED_OFFSET,
                    TYPE_INFO_PROJECT_NAME_OFFSET,
                    0x0,
                ],
            )
            .map_err(|_| SimpleError::from("failed to read name of project"))?
            .validate_utf8()?
            .to_owned();
        asr::print_message(&format!("  => found project {name} at address {address}"));

        let mut classes = HashMap::new();
        let base_class_addr: Address = process
            .read_pointer_path::<u64>(
                address,
                asr::PointerSize::Bit64,
                &[
                    TYPE_INFO_PROJECT_TYPE_INFO_GENERATED_OFFSET,
                    TYPE_INFO_PROJECT_CLASSES_OFFSET,
                ],
            )
            .map_err(|_| SimpleError::from("failed to get the base class address"))?
            .into();
        let num_classes = process
            .read_pointer_path::<u32>(
                address,
                asr::PointerSize::Bit64,
                &[
                    TYPE_INFO_PROJECT_TYPE_INFO_GENERATED_OFFSET,
                    TYPE_INFO_PROJECT_NUM_CLASSES_OFFSET,
                ],
            )
            .map_err(|_| SimpleError::from("failed to get number of classes"))?
            as u64;
        asr::print_message(&format!("    => found {num_classes} classes"));
        for class_index in 0u64..num_classes {
            let class_addr = base_class_addr + class_index * CLASS_TYPE_INFO_SIZE;
            let class_val = process
                .read::<u64>(class_addr)
                .map_err(|_| SimpleError::from("failed to read value at class address"))?;
            if class_val == 0 {
                break;
            }
            let class = ClassTypeInfo::init(process, class_addr)?;

            classes.insert(class.name.clone(), class);
        }
        asr::print_message("    => finished preloading those classes");

        Ok(TypeInfoProject {
            _process: process,
            _address: address,

            name,
            classes,
        })
    }

    pub fn get_class(&self, class_name: &str) -> Option<&ClassTypeInfo<'a>> {
        self.classes.get(class_name)
    }
}
