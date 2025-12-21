use crate::error::SimpleError;
use crate::watchers::pointer_path::PointerPathReadable;
#[cfg(feature = "gba")]
use asr::emulator::gba::Emulator;
use asr::{Address, PointerSize};
use bytemuck::CheckedBitPattern;
use std::error::Error;

#[cfg(feature = "gba")]
impl PointerPathReadable for Emulator {
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
