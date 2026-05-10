use crate::error::SimpleError;
use asr::string::{ArrayCString, ArrayWString};
use bytemuck::CheckedBitPattern;
use once_cell::unsync::OnceCell;
use std::error::Error;

pub mod gba;
pub mod pointer_path;
pub mod unity;

/// Simply something that, upon request, tries to retrieve a value
///
/// This can be anything really, but the canonical case is something that will try to read a value
/// from memory.
pub trait ValueGetter<T> {
    fn get(&self) -> Result<T, Box<dyn Error>>;
}

impl<T: Clone> ValueGetter<T> for T {
    fn get(&self) -> Result<T, Box<dyn Error>> {
        Ok(self.clone())
    }
}

/// A Watcher is used for fetching values, and comparing those values against old versions of that
/// value.
///
/// It will:
/// 1. Fetch a value (with the given ValueGetter) when asked with current()
/// 2. Cache that value and return it on future calls to current()
/// 3. Invalidate that cache on calls to invalidate()
/// 4. Return the latest invalidated version of that value in old()
///
/// In typical usage, you fetch the value whenever it's needed at the beginning of every tick (e.g.
/// read a value from memory), and invalidate it at the end of the tick. During the tick, you can
/// then observe how that value changed from the previous tick, and act on that behaviour (e.g.
/// value went from false -> true, I should do something, i.e. start/pause/split the timer)
pub struct Watcher<'a, T: Clone> {
    source: Box<dyn ValueGetter<T> + 'a>,
    current: OnceCell<T>,
    old: Option<T>,
    default: Option<T>,
}

impl<'a, T: Clone> Watcher<'a, T> {
    pub fn new(source: Box<dyn ValueGetter<T> + 'a>) -> Self {
        Self {
            source,
            current: OnceCell::new(),
            old: None,
            default: None,
        }
    }

    /// Try to read from the given source a fresh value. This value will be cached until that cache
    /// is invalidated by `invalidate`.
    pub fn current(&self) -> Result<T, Box<dyn Error>> {
        self.current
            .get_or_try_init(|| {
                // If we retrieve the value successfully, return it outright
                let err = match self.source.get() {
                    Ok(value) => return Ok(value),
                    Err(e) => e,
                };

                // Only return the error we got if we have no default
                match self.default.clone() {
                    Some(default) => Ok(default),
                    None => Err(err),
                }
            })
            .cloned()
    }

    /// Retrieve the previous value for this watcher.
    ///
    /// The "previous" value refers to the value retrieved from `current` (if any) prior to the most
    /// recent `invalidate` call.
    ///
    /// This returns `None` if there's no value to retrieve, otherwise returns `Some` if it can.
    pub fn old(&self) -> Option<T> {
        self.old.clone()
    }

    /// Invalidate the watcher. This moves the value of `current` into `old`, and empties the cache
    /// for `current`, meaning the next call to `current()` will get a fresh value.
    pub fn invalidate(&mut self) {
        // The implication of this is if we had a value for old, but did not read current again
        // before calling invalidate, old becomes None instead of using the already held value.
        // Is this desirable?
        // None of my code depends on this behaviour at the moment, since everything is read on
        // every tick anyway.
        self.old = self.current.get().cloned();
        self.current = OnceCell::new();
    }

    /// When we fail to get the value, use the given default value instead of failing the call to
    /// `current()`.
    ///
    /// This swallows the error from `current()`, but may be desirable over dealing with error cases.
    pub fn default_given(self, default: T) -> Self {
        Watcher {
            source: self.source,
            current: self.current,
            old: self.old,
            default: Some(default),
        }
    }
}

impl<T: Copy + 'static> Watcher<'_, T> {
    /// Creates a watcher that will always return the same value.
    ///
    /// Usually used as a placeholder.
    pub fn constant(constant: T) -> Self {
        Self::new(Box::new(constant))
    }
}

impl<'a, T: Clone + PartialEq> Watcher<'a, T> {
    /// Simply tells you if the value changed. Requires reading the value from current, so this can
    /// return an Err.
    pub fn changed(&self) -> Result<bool, Box<dyn Error>> {
        match self.old.clone() {
            None => Ok(false),
            Some(old) => Ok(old != self.current()?),
        }
    }

    pub fn was(&self, value: T) -> bool {
        self.old.clone().is_some_and(|old| old == value)
    }

    pub fn is(&self, value: T) -> Result<bool, Box<dyn Error>> {
        Ok(self.current()? == value)
    }

    pub fn changed_from(&self, from: T) -> Result<bool, Box<dyn Error>> {
        Ok(self.was(from) && self.changed()?)
    }

    pub fn changed_to(&self, to: T) -> Result<bool, Box<dyn Error>> {
        Ok(self.changed()? && self.is(to)?)
    }

    pub fn changed_from_to(&self, from: T, to: T) -> Result<bool, Box<dyn Error>> {
        Ok(self.was(from) && self.is(to)?)
    }
}

impl<'a, T: CheckedBitPattern + Default> Watcher<'a, T> {
    /// Use the default value of `T` for the default when `current()` fails to retrieve a new value.
    ///
    /// See `default_given` for more documentation.
    pub fn default(self) -> Self {
        Watcher {
            source: self.source,
            current: self.current,
            old: self.old,
            default: Some(T::default()),
        }
    }
}

impl<'a, T: Clone + PartialEq + 'a> From<T> for Watcher<'a, T> {
    fn from(value: T) -> Self {
        Watcher::new(Box::new(value))
    }
}

impl<'a, const N: usize> Watcher<'a, ArrayCString<N>> {
    pub fn current_string(&self) -> Result<String, Box<dyn Error>> {
        let curr = self.current()?;
        let str = curr.validate_utf8()?.to_owned();

        Ok(str)
    }
    pub fn old_string(&self) -> Result<String, Box<dyn Error>> {
        match self.old {
            None => Err(SimpleError::from("No old value").into()),
            Some(cstr) => Ok(cstr.validate_utf8()?.to_owned()),
        }
    }
}

impl<'a, const N: usize> Watcher<'a, ArrayWString<N>> {
    pub fn current_string(&self) -> Result<String, Box<dyn Error>> {
        let curr = self.current()?;
        let str = String::from_utf16(curr.as_slice())?;

        Ok(str)
    }
    pub fn old_string(&self) -> Result<String, Box<dyn Error>> {
        match self.old {
            None => Err(SimpleError::from("No old value").into()),
            Some(wstr) => Ok(String::from_utf16(wstr.as_slice())?),
        }
    }
}
