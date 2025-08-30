use crate::error::SimpleError;
#[cfg(feature = "gba")]
use asr::emulator::gba::Emulator;
use asr::{Address, PointerSize, Process};
use bytemuck::CheckedBitPattern;
use once_cell::unsync::OnceCell;
use std::error::Error;
use std::iter::once;

pub trait Readable {
    fn read_pointer_path<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        pointer_size: PointerSize,
        path: &[u64],
    ) -> Result<T, Box<dyn Error>>;
}

impl Readable for Process {
    fn read_pointer_path<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        pointer_size: PointerSize,
        path: &[u64],
    ) -> Result<T, Box<dyn Error>> {
        self.read_pointer_path::<T>(address, pointer_size, &path)
            .map_err(|_| SimpleError::from("unable to read value from pointer path").into())
    }
}

#[cfg(feature = "gba")]
impl Readable for Emulator {
    fn read_pointer_path<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        _pointer_size: PointerSize,
        path: &[u64],
    ) -> Result<T, Box<dyn Error>> {
        let path = path.iter().map(|o| *o as u32).collect::<Vec<u32>>();
        let path = path.as_slice();
        self.read_pointer_path::<T>(address.into().value() as u32, path)
            .map_err(|_| SimpleError::from("unable to read value from pointer path").into())
    }
}

pub struct PointerPath<'a, R: Readable + ?Sized> {
    name: Option<String>,
    readable: &'a R,
    base_address: Address,
    path: Vec<u64>,
    pointer_size: PointerSize,
}

impl<'a, R: Readable + ?Sized> PointerPath<'a, R> {
    pub fn new(
        readable: &'a R,
        base_address: impl Into<Address>,
        path: impl Into<Vec<u64>>,
    ) -> Self {
        PointerPath {
            name: None,
            readable,
            pointer_size: PointerSize::Bit64,
            base_address: base_address.into(),
            path: path.into(),
        }
    }

    pub fn new32(
        readable: &'a R,
        base_address: impl Into<Address>,
        path: impl Into<Vec<u64>>,
    ) -> Self {
        PointerPath {
            name: None,
            readable,
            pointer_size: PointerSize::Bit32,
            base_address: base_address.into(),
            path: path.into(),
        }
    }

    pub fn named<T: Into<String>>(self, name: T) -> Self {
        PointerPath {
            name: Some(name.into()),
            readable: self.readable,
            pointer_size: self.pointer_size,
            base_address: self.base_address,
            path: self.path,
        }
    }

    // the first offset of the child path should not dereference
    pub fn child(&self, path: impl Into<Vec<u64>>) -> Self {
        // im so dumb dude i dontcare shut up
        let (original_last, original_prefix) = self.path.split_last().unwrap_or((&0, &[]));
        let path = path.into();
        let (child_prefix, rest) = path.split_first().expect("child path is empty");
        let child_prefix = *child_prefix;
        let original_last = *original_last;
        let new_middle_offset = original_last + child_prefix;

        PointerPath {
            name: None,
            readable: self.readable,
            pointer_size: self.pointer_size,
            base_address: self.base_address,
            path: original_prefix
                .iter()
                .copied()
                .chain(once(new_middle_offset))
                .chain(rest.to_owned())
                .collect::<Vec<_>>(),
        }
    }

    pub fn child_watcher<T: CheckedBitPattern>(
        &self,
        path: impl Into<Vec<u64>>,
    ) -> MemoryWatcher<'a, R, T> {
        self.child(path).into()
    }

    pub fn read<T: CheckedBitPattern>(&self) -> Result<T, Box<dyn Error>> {
        let valid_path = if !self.path.is_empty() {
            &self.path
        } else {
            &[0x0].into()
        };

        self.readable
            .read_pointer_path(self.base_address, self.pointer_size, valid_path)
            .map_err(|e| {
                let pointer_name = self.name.clone().unwrap_or(format!(
                    "unnamed pointer path (0x{:?}, {:?})",
                    self.base_address, valid_path
                ));
                SimpleError::wrap(format!("error while reading {pointer_name}"), e).into()
            })
    }
}

pub trait Invalidatable {
    fn invalidate(&mut self);
}

pub struct MemoryWatcher<'a, R: Readable + ?Sized, T: CheckedBitPattern> {
    path: PointerPath<'a, R>,
    current: OnceCell<T>,
    old: Option<T>,
    default: Option<T>,
}

impl<'a, R: Readable + ?Sized, T: CheckedBitPattern> MemoryWatcher<'a, R, T> {
    pub fn new(readable: &'a R, base_address: Address, path: impl Into<Vec<u64>>) -> Self {
        MemoryWatcher {
            path: PointerPath::new(readable, base_address, path),
            current: OnceCell::new(),
            old: None,
            default: None,
        }
    }

    pub fn default_given(self, default: T) -> Self {
        MemoryWatcher {
            path: self.path,
            current: self.current,
            old: self.old,
            default: Some(default),
        }
    }

    pub fn current(&self) -> Result<T, Box<dyn Error>> {
        self.current
            .get_or_try_init(|| {
                let err = match self.path.read() {
                    Ok(value) => return Ok(value),
                    Err(e) => e,
                };

                match self.default {
                    Some(default) => Ok(default),
                    None => Err(err),
                }
            })
            .copied()
    }

    pub fn old(&self) -> Option<T> {
        self.old
    }

    pub fn child(&self, path: impl Into<Vec<u64>>) -> Self {
        self.path.child(path).into()
    }
}

impl<'a, R: Readable + ?Sized, T: CheckedBitPattern + PartialEq> MemoryWatcher<'a, R, T> {
    pub fn changed(&self) -> Result<bool, Box<dyn Error>> {
        match self.old {
            None => Ok(true),
            Some(old) => Ok(old != self.current()?),
        }
    }
}

impl<'a, R: Readable + ?Sized, T: CheckedBitPattern + Default> MemoryWatcher<'a, R, T> {
    pub fn default(self) -> Self {
        MemoryWatcher {
            path: self.path,
            current: self.current,
            old: self.old,
            default: Some(T::default()),
        }
    }
}

impl<'a, R: Readable + ?Sized, T: CheckedBitPattern> Invalidatable for MemoryWatcher<'a, R, T> {
    fn invalidate(&mut self) {
        self.old = self.current.get().copied();
        self.current = OnceCell::new();
    }
}

impl<'a, R: Readable + ?Sized, T: CheckedBitPattern> From<PointerPath<'a, R>>
    for MemoryWatcher<'a, R, T>
{
    fn from(value: PointerPath<'a, R>) -> Self {
        MemoryWatcher::new(value.readable, value.base_address, value.path)
    }
}
