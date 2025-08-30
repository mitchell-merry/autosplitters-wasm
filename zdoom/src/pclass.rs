use std::{cmp::Ordering, collections::HashMap, rc::Rc};

use asr::{string::ArrayCString, Address, Error, Process};
use bitflags::bitflags;
use once_cell::unsync::OnceCell;
use regex::Regex;

use super::{name_manager::NameManager, tarray::TArray, Memory};

const DOBJECT_CLASS: u64 = 0x8;

const PFIELD_NAME: u64 = 0x28;
const PFIELD_OFFSET: u64 = 0x38;
const PFIELD_TYPE: u64 = 0x40;
const PFIELD_FLAGS: u64 = 0x48;

const PCLASS_SIZE: u64 = 0x30;
const PCLASS_TYPENAME: u64 = 0x38;
const PCLASS_PTYPE: u64 = 0x90;

const PTYPE_SIZE: u64 = 0xC;
const PTYPE_ALIGN: u64 = 0x10;
const PTYPE_FLAGS: u64 = 0x14;
const PTYPE_DESCRIPTIVE_NAME: u64 = 0x48;

#[derive(Clone)]
pub struct PClass<'a> {
    process: &'a Process,
    memory: Rc<Memory>,
    name_manager: Rc<NameManager<'a>>,
    address: Address,

    size: OnceCell<u32>,
    name: OnceCell<String>,
    ptype: OnceCell<PType<'a>>,
    fields: OnceCell<HashMap<String, PField<'a>>>, // does not contain fields of superclasses
}

impl<'a> PClass<'a> {
    pub fn new(
        process: &'a Process,
        memory: Rc<Memory>,
        name_manager: Rc<NameManager<'a>>,
        address: Address,
    ) -> PClass<'a> {
        PClass {
            process,
            name_manager,
            memory,
            address,

            size: OnceCell::new(),
            name: OnceCell::new(),
            ptype: OnceCell::new(),
            fields: OnceCell::new(),
        }
    }

    pub fn size(&self) -> Result<&u32, Error> {
        self.size
            .get_or_try_init(|| self.process.read(self.address + PCLASS_SIZE))
    }

    pub fn name(&self) -> Result<&String, Error> {
        self.name.get_or_try_init(|| {
            self.name_manager
                .get_chars(self.process.read::<u32>(self.address + PCLASS_TYPENAME)?)
        })
    }

    pub fn ptype(&self) -> Result<&PType<'a>, Error> {
        self.ptype.get_or_try_init(|| {
            let address = self.process.read::<u64>(self.address + PCLASS_PTYPE)?;

            Ok(PType::new(self.process, address.into()))
        })
    }

    pub fn fields<'b>(&'b self) -> Result<&'b HashMap<String, PField<'a>>, Error> {
        self.fields.get_or_try_init(|| {
            let mut fields = HashMap::new();

            let fields_addr = self.address.add(self.memory.offsets.pclass_fields);
            let field_addrs = TArray::new(self.process, fields_addr);

            for field_addr in field_addrs.iter::<u64>()? {
                let field = PField::new(
                    self.process,
                    self.memory.clone(),
                    self.name_manager.clone(),
                    field_addr.into(),
                );
                fields.insert(field.name().map(|f| f.to_owned())?, field);
            }

            Ok(fields)
        })
    }

    pub fn show_class(&self) -> Result<String, Error> {
        let mut struct_out = String::new();

        let class_size = self.size()?.to_owned();
        struct_out.push_str(&format!("// size: 0x{class_size:X}\n"));

        let parent_class: Address = self
            .process
            .read_pointer_path::<u64>(self.address, asr::PointerSize::Bit64, &[0x0])?
            .into();

        struct_out.push_str(&format!("class {} ", self.name()?));

        if parent_class != Address::NULL {
            let parent_class = PClass::new(
                self.process,
                self.memory.clone(),
                self.name_manager.clone(),
                parent_class,
            );
            struct_out.push_str(&format!(": public {} ", parent_class.name()?));
        }

        struct_out.push_str("{\n");

        let fields = self.fields()?;
        let mut sorted_fields = Vec::<PField<'a>>::new();

        for field in fields.values() {
            sorted_fields.push(field.to_owned());
        }

        sorted_fields.sort();

        for field in sorted_fields {
            let ptype = field.ptype()?;
            let field_flags = field.flags()?;
            let modifier = if field_flags.contains(PFieldFlags::Static) {
                "static "
            } else {
                ""
            };

            let ptype_flags = ptype.flags()?;
            let struct_modifier = if ptype_flags.contains(TypeFlags::Container) {
                "struct "
            } else {
                ""
            };

            let pointer_modifier = if ptype_flags.contains(TypeFlags::Pointer)
                || ptype_flags.contains(TypeFlags::ClassPointer)
                || ptype_flags.contains(TypeFlags::ObjectPointer)
            {
                "*"
            } else {
                ""
            };

            let ptype_name = ptype.name()?;
            let n =
                PType::name_as_field_type(ptype_name.to_owned()).unwrap_or(ptype_name.to_owned());

            struct_out.push_str(&format!(
                "  {}{}{} {}{}; // raw type name: {ptype_name}, offset: 0x{:X}, size: 0x{:X}, align: 0x{:X} ({:?}, {:?})\n",
                modifier,
                struct_modifier,
                n,
                pointer_modifier,
                field.name()?,
                field.offset()?,
                field.ptype()?.size()?,
                field.ptype()?.align()?,
                field_flags,
                ptype_flags,
            ))
        }

        struct_out.push_str("};");

        Ok(struct_out)
    }
}

bitflags! {
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct PFieldFlags: u32 {
        const Optional = 1 << 0;        // func param is optional
        const Method = 1 << 1;          // func has an implied self parameter
        const Action = 1 << 2;          // func has implied owner and state parameters
        const Native = 1 << 3;          // func is native code, field is natively defined
        const ReadOnly = 1 << 4;        // field is read only, do not write to it
        const Private = 1 << 5;         // field is private to containing class
        const Protected = 1 << 6;       // field is only accessible by containing class and children.
        const Deprecated = 1 << 7;      // Deprecated fields should output warnings when used.
        const Virtual = 1 << 8;         // function is virtual
        const Final = 1 << 9;           // Function may not be overridden in subclasses
        const In = 1 << 10;
        const Out = 1 << 11;
        const Implicit = 1 << 12;       // implicitly created parameters (i.e. do not compare types when checking function signatures)
        const Static = 1 << 13;
        const InternalAccess = 1 << 14; // overrides ReadOnly for internal script code.
        const Override = 1 << 15;       // overrides a virtual function from the parent class.
        const Ref = 1 << 16;            // argument is passed by reference.
        const Transient = 1 << 17;      // don't auto serialize field.
        const Meta = 1 << 18;           // static class data (by necessity read only.)
        const VarArg = 1 << 19;         // [ZZ] vararg: don't typecheck values after ... in function signature
        const UI = 1 << 20;             // [ZZ] ui: object is ui-scope only (can't modify playsim)
        const Play = 1 << 21;           // [ZZ] play: object is playsim-scope only (can't access ui)
        const VirtualScope= 1 << 22;    // [ZZ] virtualscope: object should use the scope of the particular class it's being used with (methods only)
        const ClearScope = 1 << 23;     // [ZZ] clearscope: this method ignores the member access chain that leads to it and is always plain data.
    }
}

#[derive(Clone)]
pub struct PField<'a> {
    process: &'a Process,
    memory: Rc<Memory>,
    name_manager: Rc<NameManager<'a>>,
    address: Address,

    class: OnceCell<PClass<'a>>,
    name: OnceCell<String>,
    offset: OnceCell<u32>,
    ptype: OnceCell<PType<'a>>,
    flags: OnceCell<PFieldFlags>,
}

impl<'a> PField<'a> {
    pub fn new(
        process: &'a Process,
        memory: Rc<Memory>,
        name_manager: Rc<NameManager<'a>>,
        address: Address,
    ) -> PField<'a> {
        PField {
            process,
            memory,
            name_manager,
            address,
            class: OnceCell::new(),
            name: OnceCell::new(),
            offset: OnceCell::new(),
            ptype: OnceCell::new(),
            flags: OnceCell::new(),
        }
    }

    pub fn class(&self) -> Result<&PClass<'a>, Error> {
        self.class.get_or_try_init(|| {
            let class_addr: Address = self
                .process
                .read::<u64>(self.address + DOBJECT_CLASS)?
                .into();

            Ok(PClass::new(
                self.process,
                self.memory.clone(),
                self.name_manager.clone(),
                class_addr,
            ))
        })
    }

    pub fn name(&self) -> Result<&String, Error> {
        self.name.get_or_try_init(|| {
            let name_index: u32 = self.process.read(self.address + PFIELD_NAME)?;

            self.name_manager.get_chars(name_index)
        })
    }

    pub fn offset(&self) -> Result<&u32, Error> {
        self.offset
            .get_or_try_init(|| self.process.read(self.address + PFIELD_OFFSET))
    }

    pub fn ptype(&self) -> Result<&PType<'a>, Error> {
        self.ptype.get_or_try_init(|| {
            let ptype_address: Address =
                self.process.read::<u64>(self.address + PFIELD_TYPE)?.into();

            Ok(PType::new(self.process, ptype_address))
        })
    }

    pub fn flags(&self) -> Result<&PFieldFlags, Error> {
        self.flags.get_or_try_init(|| {
            Ok(PFieldFlags::from_bits_truncate(
                self.process.read::<u32>(self.address + PFIELD_FLAGS)?,
            ))
        })
    }
}

impl<'a> PartialEq for PField<'a> {
    fn eq(&self, other: &Self) -> bool {
        let flags = self.flags();
        let other_flags = other.flags();
        if flags.is_err() || other_flags.is_err() {
            return false;
        }

        let name = self.name();
        let other_name = other.name();
        if name.is_err() || other_name.is_err() {
            return false;
        }

        name.unwrap() == other_name.unwrap() || flags.unwrap() == other_flags.unwrap()
    }
}

impl<'a> Eq for PField<'a> {}

impl<'a> PartialOrd for PField<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for PField<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        let flags = self.flags().map(|f| f.to_owned()).unwrap_or_default();
        let other_flags = other.flags().map(|f| f.to_owned()).unwrap_or_default();

        if flags.contains(PFieldFlags::Static) && !other_flags.contains(PFieldFlags::Static) {
            return Ordering::Less;
        } else if !flags.contains(PFieldFlags::Static) && other_flags.contains(PFieldFlags::Static)
        {
            return Ordering::Greater;
        }

        let offset = self.offset().map(|o| o.to_owned()).unwrap_or_default();
        let other_offset = other.offset().map(|o| o.to_owned()).unwrap_or_default();

        if offset < other_offset {
            return Ordering::Less;
        } else if offset > other_offset {
            return Ordering::Greater;
        }

        Ordering::Equal
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct TypeFlags: u32 {
        const Scalar = 1 << 0;
        const Container = 1 << 1;
        const Int = 1 << 2;
        const IntNotInt = 1 << 3;
        const Float = 1 << 4;
        const Pointer = 1 << 5;
        const ObjectPointer = 1 << 6;
        const ClassPointer = 1 << 7;
        const Array = 1 << 8;
    }
}

#[derive(Clone)]
pub struct PType<'a> {
    process: &'a Process,
    pub address: Address,

    size: OnceCell<u32>,
    align: OnceCell<u32>,
    flags: OnceCell<TypeFlags>,
    name: OnceCell<String>,
}

impl<'a> PType<'a> {
    pub fn new(process: &'a Process, address: Address) -> PType<'a> {
        PType {
            process,
            address,
            size: OnceCell::new(),
            align: OnceCell::new(),
            flags: OnceCell::new(),
            name: OnceCell::new(),
        }
    }

    pub fn size(&self) -> Result<&u32, Error> {
        self.size
            .get_or_try_init(|| self.process.read::<u32>(self.address.add(PTYPE_SIZE)))
    }

    pub fn align(&self) -> Result<&u32, Error> {
        self.align
            .get_or_try_init(|| self.process.read::<u32>(self.address.add(PTYPE_ALIGN)))
    }

    pub fn flags(&self) -> Result<&TypeFlags, Error> {
        self.flags.get_or_try_init(|| {
            Ok(TypeFlags::from_bits_truncate(
                self.process.read(self.address + PTYPE_FLAGS)?,
            ))
        })
    }

    pub fn name(&self) -> Result<&String, Error> {
        self.name.get_or_try_init(|| {
            let c_str = self.process.read_pointer_path::<ArrayCString<128>>(
                self.address,
                asr::PointerSize::Bit64,
                &[PTYPE_DESCRIPTIVE_NAME, 0x0],
            );

            let b = c_str?
                .validate_utf8()
                .expect("name should always be utf-8")
                .to_owned();

            Ok(b)
        })
    }

    pub fn name_as_field_type(name: String) -> Result<String, regex::Error> {
        let generic =
            Regex::new(r"^(?<outer_type>.+?)<(?<inner_type>.+?)>(?<elements>\[\d+\])?$").unwrap();

        if let Some(captures) = generic.captures(name.as_str()) {
            let outer_type = captures["outer_type"].to_owned();
            let elements = captures.name("elements");
            let mut inner_type = PType::name_as_field_type(captures["inner_type"].to_owned())?;
            if let Some(elements) = elements {
                inner_type = format!("{}{}", inner_type, elements.as_str());
            }

            Ok(match outer_type.as_str() {
                "DynArray" => format!("TArray<{inner_type}>"),
                "Struct" => inner_type,
                "NativeStruct" => inner_type,
                "Class" => inner_type,
                "Pointer" => inner_type,
                "ClassPointer" => inner_type,
                _ => format!("{outer_type}<{inner_type}>"),
            })
        } else {
            Ok(match name.as_str() {
                "Bool" => "bool",
                "Float4" => "float",
                "Float8" => "double",
                "String" => "char*",
                "SInt1" => "int8_t",
                "SInt2" => "int16_t",
                "SInt4" => "int32_t",
                "UInt1" => "uint8_t",
                "UInt2" => "uint16_t",
                "UInt4" => "uint32_t",
                x => x,
            }
            .to_owned())
        }
    }
}
