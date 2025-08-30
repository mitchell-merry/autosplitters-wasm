use std::rc::Rc;

use crate::name_manager::NameManager;
use crate::pclass::PClass;
use crate::tarray::TArray;
use asr::{print_message, string::ArrayCString, Address, Error, Process};

use super::Memory;

pub struct Level<'a> {
    process: &'a Process,
    memory: Rc<Memory>,
    name_manager: Rc<NameManager<'a>>,
    address: Address,
    _name: Option<String>,
}

impl<'a> Level<'a> {
    pub fn new(
        process: &'a Process,
        memory: Rc<Memory>,
        name_manager: Rc<NameManager<'a>>,
        address: Address,
    ) -> Level<'a> {
        Level {
            process,
            memory,
            name_manager,
            address,
            _name: None,
        }
    }

    pub fn invalidate_cache(&mut self) {
        self._name = None
    }

    pub fn name(&mut self) -> Result<&str, Error> {
        if let Some(ref name) = self._name {
            return Ok(name);
        }

        let c_str = self.process.read_pointer_path::<ArrayCString<128>>(
            self.address,
            asr::PointerSize::Bit64,
            &[self.memory.offsets.level_mapname, 0x0],
        )?;

        let name = c_str
            .validate_utf8()
            .unwrap_or_else(|_| panic!("name should always be utf-8. raw: {:?}", c_str.as_bytes()))
            .to_owned();

        self._name = Some(name.clone());

        Ok(self._name.as_ref().unwrap())
    }

    pub fn find_actor(&self, actor_name: &str) -> Result<Address, Option<Error>> {
        let sectors = TArray::new(
            self.process,
            self.address + self.memory.offsets.level_sectors,
        );

        for sector in sectors.iter_addr(0x310)? {
            let mut actor_next = sector + self.memory.offsets.sector_thinglist;
            while let Ok(actor) = self.process.read::<u64>(actor_next) {
                if Address::from(actor) == Address::NULL {
                    break;
                }

                let class = self.process.read::<u64>(actor + 0x8)?.into();
                let class = PClass::new(
                    self.process,
                    self.memory.clone(),
                    self.name_manager.clone(),
                    class,
                );

                let name = class.name()?;
                if name == actor_name {
                    return Ok(actor.into());
                }

                actor_next = Address::from(actor + 0x40);
            }
        }

        Err(None)
    }

    pub fn get_actor_names(&self, actor_class: &PClass<'a>) -> Result<Vec<String>, Error> {
        let sectors = TArray::new(
            self.process,
            self.address + self.memory.offsets.level_sectors,
        );

        let mut actors = Vec::new();

        for sector in sectors.iter_addr(0x310)? {
            let mut actor_next = sector + self.memory.offsets.sector_thinglist;
            while let Ok(actor) = self.process.read::<u64>(actor_next) {
                if Address::from(actor) == Address::NULL {
                    break;
                }

                let class = self.process.read::<u64>(actor + 0x8)?.into();
                let class = PClass::new(
                    self.process,
                    self.memory.clone(),
                    self.name_manager.clone(),
                    class,
                );

                let name = class.name()?;
                actors.push(name.to_owned());

                actor_next = Address::from(
                    actor
                        + actor_class
                            .fields()
                            .unwrap()
                            .get("snext")
                            .unwrap()
                            .offset()
                            .unwrap()
                            .to_owned() as u64,
                );
            }
        }

        Ok(actors)
    }

    pub fn dump_actors(&self, actor_class: &PClass<'a>) -> Result<(), Error> {
        print_message("Dumping actors...");
        let actors = self.get_actor_names(actor_class)?;
        actors.iter().for_each(|actor| asr::print_message(actor));

        Ok(())
    }
}
