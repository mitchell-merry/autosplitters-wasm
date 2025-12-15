use crate::error::SimpleError;
use crate::watchers::{ValueGetter, Watcher};
use asr::game_engine::unity::mono::{Image, Module, UnityPointer};
use asr::Process;
use bytemuck::CheckedBitPattern;
use std::error::Error;

#[cfg(feature = "unity")]
pub struct UnityImage<'a> {
    pub process: &'a Process,
    pub module: &'a Module,
    pub image: &'a Image,
}

impl<'a> UnityImage<'a> {
    pub fn new(process: &'a Process, module: &'a Module, image: &'a Image) -> Self {
        UnityImage {
            process,
            module,
            image,
        }
    }

    pub fn path(
        &self,
        class_name: &'static str,
        nr_of_parents: usize,
        fields: &[&'static str],
    ) -> UnityPointerPath<'a> {
        UnityPointerPath {
            process: self.process,
            module: self.module,
            image: self.image,
            pointer: UnityPointer::new(class_name, nr_of_parents, fields),
        }
    }
}

#[cfg(feature = "unity")]
pub struct UnityPointerPath<'a> {
    process: &'a Process,
    module: &'a Module,
    image: &'a Image,
    pointer: UnityPointer<128>,
}

impl<'a, T: CheckedBitPattern> ValueGetter<T> for UnityPointerPath<'a> {
    fn get(&self) -> Result<T, Box<dyn Error>> {
        self.pointer
            .deref(self.process, self.module, self.image)
            .map_err(|_| SimpleError::from("unable to read unity pointer").into())
    }
}

impl<'a, T: CheckedBitPattern> From<UnityPointerPath<'a>> for Watcher<'a, T> {
    fn from(value: UnityPointerPath<'a>) -> Self {
        Watcher::new(Box::new(value))
    }
}
