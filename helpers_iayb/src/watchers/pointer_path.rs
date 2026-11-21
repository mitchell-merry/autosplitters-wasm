use crate::error::SimpleError;
use crate::watchers::{ValueGetter, Watcher};
use asr::{Address, PointerSize, Process};
use bytemuck::CheckedBitPattern;
use std::error::Error;
use std::fmt::Display;
use std::iter::once;

/// A PointerPathReadable is something that can read a value by pointer path.
///
/// The two cases we deal with primarily are Process (the generic asr struct for a process) and
/// Emulator (the struct that deals strictly with emulators, with various builtins).
///
/// Our code should not assume it has a particular kind of readable object.
pub trait PointerPathReadable {
    fn read_pointer_path<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        pointer_size: PointerSize,
        path: &[u64],
    ) -> Result<T, Box<dyn Error>>;
}

impl PointerPathReadable for Process {
    fn read_pointer_path<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        pointer_size: PointerSize,
        path: &[u64],
    ) -> Result<T, Box<dyn Error>> {
        self.read_pointer_path::<T>(address, pointer_size, path)
            .map_err(|_| SimpleError::from("unable to read value from pointer path").into())
    }
}

/// PointerPath represents a "pointer path". This is a base address, and then a sequence of offsets
/// that should be followed to reach some final address/value.
///
/// A pointer path consists of a base address (usually calculated from a module address + an offset).
/// This address will then either point at a value, or at another pointer.
///
/// The "path" is a list of offsets. The first offset is added to the pointer read from the base
/// address, which then either points at the final value (if we are at the end of the path), or at
/// another pointer. If there are more elements in the path, we continue this process of adding
/// offsets and resolving pointers until we reach the end.
pub struct PointerPath<'a, TReadable: PointerPathReadable> {
    name: Option<String>,
    readable: &'a TReadable,
    base_address: Address,
    path: Vec<u64>,
    pointer_size: PointerSize,
}

impl<'a, TReadable: PointerPathReadable> PointerPath<'a, TReadable> {
    pub fn new(
        readable: &'a TReadable,
        base_address: impl Into<Address>,
        pointer_size: PointerSize,
        path: impl Into<Vec<u64>>,
    ) -> Self {
        Self {
            name: None,
            readable,
            base_address: base_address.into(),
            path: path.into(),
            pointer_size,
        }
    }

    /// Naming a pointer path is useful when interpreting logs.
    pub fn named<T: Into<String>>(self, name: T) -> Self {
        Self {
            name: Some(name.into()),
            readable: self.readable,
            pointer_size: self.pointer_size,
            base_address: self.base_address,
            path: self.path,
        }
    }

    /// Reads the value pointed to by the pointer path.
    ///
    /// The actual work for dereferencing and doing the reading is handled by the `readable`.
    pub fn read<T: CheckedBitPattern>(&self) -> Result<T, Box<dyn Error>> {
        let valid_path = if !self.path.is_empty() {
            &self.path
        } else {
            &vec![0x0]
        };

        self.readable
            .read_pointer_path(self.base_address, self.pointer_size, valid_path)
            .map_err(|e| SimpleError::wrap(format!("failed to read pointer path {self}"), e).into())
    }

    /// Create a new pointer path, by attaching to the end of this pointer path.
    ///
    /// For example, if you have an instance of an object at base, offset1, offset2, you may wish to
    /// access a field at offset3.
    ///
    /// You can do this by calling `my_object_path.child([offset3])`;
    /// TODO: document struct vs instance behaviour
    pub fn child(&self, path: impl Into<Vec<u64>>) -> Self {
        // im so dumb dude i dontcare shut up
        let (original_last, original_prefix) = self.path.split_last().unwrap_or((&0, &[]));
        let path = path.into();
        let (child_prefix, rest) = path.split_first().expect("child path is empty");
        let child_prefix = *child_prefix;
        let original_last = *original_last;
        // the first offset of the child path should not dereference
        let new_middle_offset = original_last + child_prefix;

        Self {
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

    pub fn child_watcher<T: CheckedBitPattern>(&self, path: impl Into<Vec<u64>>) -> Watcher<'a, T> {
        self.child(path).into()
    }
}

impl<'a, TReadable: PointerPathReadable, T: CheckedBitPattern> ValueGetter<T>
    for PointerPath<'a, TReadable>
{
    fn get(&self) -> Result<T, Box<dyn Error>> {
        self.read::<T>()
    }
}

impl<'a, TReadable: PointerPathReadable, T: CheckedBitPattern> From<PointerPath<'a, TReadable>>
    for Watcher<'a, T>
{
    fn from(value: PointerPath<'a, TReadable>) -> Self {
        Watcher::new(Box::new(value))
    }
}

impl<'a, TReadable: PointerPathReadable> Display for PointerPath<'a, TReadable> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let offsets = self
            .path
            .iter()
            .map(|offset| format!("0x{:x}", offset))
            .collect::<Vec<_>>()
            .join(", ");
        let path = format!("0x{}, {}", self.base_address, offsets);

        if let Some(name) = &self.name {
            write!(f, "({name}: {path})")
        } else {
            write!(f, "({path})")
        }
    }
}
