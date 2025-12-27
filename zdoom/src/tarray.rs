use std::marker::PhantomData;

use asr::{Address, Error, Process};
use bytemuck::CheckedBitPattern;

pub struct TArray<'a> {
    process: &'a Process,
    address: Address,
}

impl<'a> TArray<'a> {
    pub fn new(process: &'a Process, address: Address) -> TArray<'a> {
        TArray { process, address }
    }

    /// Iterate over each item in this TArray, reading the full item each time
    pub fn iter<T: CheckedBitPattern>(&self) -> Result<TArrayIterator<'a, T>, Error> {
        let size =
            self.process
                .read_pointer_path(self.address, asr::PointerSize::Bit64, &[0x8_u64])?;
        Ok(TArrayIterator::<T> {
            _phantom: PhantomData,
            process: self.process,
            address: self.address,
            size,
            index: 0,
        })
    }

    /// Iterate over the addresses pointing at each item in this TArray
    ///
    /// This is useful when you don't want to read the full content of each item,
    /// or you don't know the exact structure of the item (e.g. it's dependent on zdoom version)
    pub fn iter_addr(&self, item_size: u64) -> Result<TArrayAddressIterator, Error> {
        TArrayAddressIterator::new(self.process, self.address, item_size)
    }
}

pub struct TArrayAddressIterator {
    item_size: u64,
    array_addr: Address,
    size: u32,
    index: u32,
}

impl TArrayAddressIterator {
    fn new(
        process: &Process,
        address: Address,
        item_size: u64,
    ) -> Result<TArrayAddressIterator, Error> {
        Ok(TArrayAddressIterator {
            item_size,
            array_addr: process.read::<u64>(address + 0x0_u64)?.into(),
            size: process.read(address + 0x8_u64)?,
            index: 0,
        })
    }
}

impl Iterator for TArrayAddressIterator {
    type Item = Address;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.size {
            return None;
        }

        let offset = self.item_size * self.index as u64;

        self.index += 1;

        Some(self.array_addr + offset)
    }
}

pub struct TArrayIterator<'a, T: CheckedBitPattern> {
    _phantom: PhantomData<T>,
    process: &'a Process,
    address: Address,
    size: u32,
    index: u32,
}

impl<'a, T: CheckedBitPattern> Iterator for TArrayIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.size {
            return None;
        }

        let offset = std::mem::size_of::<T>() * self.index as usize;
        let item = self
            .process
            .read_pointer_path(self.address, asr::PointerSize::Bit64, &[0x0, offset as u64])
            .ok();

        self.index += 1;

        item
    }
}
