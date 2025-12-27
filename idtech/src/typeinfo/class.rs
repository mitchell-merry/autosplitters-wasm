use asr::string::ArrayCString;
use asr::{Address, Address64, Process};
use helpers::error::SimpleError;
use once_cell::unsync::OnceCell;
use std::collections::HashMap;
use std::error::Error;

const CLASS_TYPE_INFO_NAME_OFFSET: u64 = 0x0;
const CLASS_TYPE_INFO_VARIABLES_OFFSET: u64 = 0x28;

const CLASS_VARIABLE_TYPE_INFO_SIZE: u64 = 0x58;
const CLASS_VARIABLE_TYPE_INFO_NAME_OFFSET: u64 = 0x10;
const CLASS_VARIABLE_TYPE_INFO_OFFSET_OFFSET: u64 = 0x18;

pub struct ClassTypeInfo<'a> {
    process: &'a Process,
    address: Address,

    pub name: String,
    _size: OnceCell<u32>,
    variables: OnceCell<HashMap<String, ClassVariableInfo<'a>>>,
}

impl<'a> ClassTypeInfo<'a> {
    pub fn init(
        process: &'a Process,
        address: Address,
    ) -> Result<ClassTypeInfo<'a>, Box<dyn Error>> {
        let name = process
            .read_pointer_path::<ArrayCString<512>>(
                address,
                asr::PointerSize::Bit64,
                &[CLASS_TYPE_INFO_NAME_OFFSET, 0x0],
            )
            .map_err(|_| SimpleError::from("failed to read name of class"))?
            .validate_utf8()?
            .to_owned();

        Ok(ClassTypeInfo {
            process,
            address,
            name,

            _size: OnceCell::new(),
            variables: OnceCell::new(),
        })
    }

    pub fn get_variables(&self) -> Result<&HashMap<String, ClassVariableInfo<'a>>, Box<dyn Error>> {
        self.variables.get_or_try_init(|| {
            let variables_base = self
                .process
                .read::<Address64>(self.address + CLASS_TYPE_INFO_VARIABLES_OFFSET)
                .map_err(|_| SimpleError::from("failed to read variables base"))?;
            let mut variables = HashMap::new();

            for i in 0.. {
                let variable_addr = variables_base + i * CLASS_VARIABLE_TYPE_INFO_SIZE;
                let is_exists = self
                    .process
                    .read::<Address64>(variable_addr)
                    .map_err(|_| SimpleError::from("failed to read variable address"))?;

                if is_exists == Address64::NULL {
                    break;
                }

                let variable = ClassVariableInfo::init(self.process, variable_addr)?;

                variables.insert(variable.name.clone(), variable);
            }

            Ok(variables)
        })
    }

    pub fn get_variable(&self, variable: &str) -> Result<&ClassVariableInfo<'a>, Box<dyn Error>> {
        let variable = self
            .get_variables()?
            .get(variable)
            .ok_or(SimpleError::from(&format!(
                "unknown variable {} in class {}",
                variable, self.name
            )))?;

        Ok(variable)
    }

    pub fn get_offset(&self, variable: &str) -> Result<u64, Box<dyn Error>> {
        Ok(*self.get_variable(variable)?.get_offset()?)
    }
}

// representation of the classVariableInfo_t class
pub struct ClassVariableInfo<'a> {
    process: &'a Process,
    address: Address64,

    pub name: String,
    offset: OnceCell<u64>,
}

impl<'a> ClassVariableInfo<'a> {
    pub fn init(
        process: &'a Process,
        address: Address64,
    ) -> Result<ClassVariableInfo<'a>, Box<dyn Error>> {
        let name = process
            .read_pointer_path::<ArrayCString<512>>(
                address,
                asr::PointerSize::Bit64,
                &[CLASS_VARIABLE_TYPE_INFO_NAME_OFFSET, 0x0],
            )
            .map_err(|_| SimpleError::from("failed to read name of variable"))?
            .validate_utf8()?
            .to_owned();

        Ok(ClassVariableInfo {
            process,
            address,
            name,

            offset: OnceCell::new(),
        })
    }

    pub fn get_offset(&self) -> Result<&u64, Box<dyn Error>> {
        self.offset.get_or_try_init(|| {
            let offset = self
                .process
                .read::<u32>(self.address + CLASS_VARIABLE_TYPE_INFO_OFFSET_OFFSET)
                .map_err(|_| {
                    SimpleError::from(&format!("failed to read offset of variable {}", self.name))
                })?;

            Ok(offset.into())
        })
    }
}
