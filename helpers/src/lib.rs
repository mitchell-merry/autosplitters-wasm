extern crate core;
extern crate proc_macro;
pub mod error;
pub mod memory;
pub mod settings;
pub mod try_load;
pub mod watchers;

use crate::error::SimpleError;
use asr::{print_message, timer};
pub use paste::paste;
use std::collections::{HashMap, HashSet};
use std::error::Error;

#[macro_export]
macro_rules! impl_auto_splitter_state {
    ($watchers:ident {
        $($field:ident : Watcher<$ty:ty>),+ $(,)?
    }) => {
        paste! {
            struct [<$watchers __State>] {
                $($field: $ty,)+
            }

            #[derive(Default)]
            struct $watchers {
                $($field: Watcher<$ty>,)+
            }

            impl $watchers {
                fn to_states(&self) -> Option<([<$watchers __State>], [<$watchers __State>])> {
                    $(let $field = self.$field.pair.as_ref()?;)+

                    let current = [<$watchers __State>] {
                        $($field: $field.current.to_owned(),)+
                    };

                    #[cfg(debug_assertions)]
                    {
                        $(timer::set_variable(stringify!($field), &format!("{:#?}", current.$field));)+
                    }

                    Some((
                        [<$watchers __State>] {
                            $($field: $field.old.to_owned(),)+
                        },
                        current,
                    ))
                }
            }
        }
    };
}

pub fn get_setting(
    key: &str,
    setting_defaults: &HashMap<String, bool>,
) -> Result<bool, Box<dyn Error>> {
    let settings_map = asr::settings::Map::load();

    if let Some(value) = settings_map.get(key) {
        return value.get_bool().ok_or(
            SimpleError::from(&format!("stored value for setting {} not a bool", key)).into(),
        );
    }

    if let Some(default_value) = setting_defaults.get(key) {
        return Ok(*default_value);
    }

    Err(SimpleError::from(&format!(
        "attempted to read value for unknown setting {key}"
    ))
    .into())
}

pub fn better_split(
    key: &String,
    setting_defaults: &HashMap<String, bool>,
    completed_splits: &mut HashSet<String>,
) -> Result<bool, Box<dyn Error>> {
    print_message(&format!("trying to split {key}"));
    if completed_splits.contains(key) {
        print_message(&format!("-> {key} already split"));
        return Ok(false);
    }

    let value = get_setting(key, setting_defaults)?;
    if !value {
        print_message(&format!("-> {key} set to false"));
        return Ok(false);
    }

    if !completed_splits.insert(key.to_owned()) {
        print_message(&format!("-> {key} already split"));
        return Ok(false);
    }

    print_message(&format!("-> {key} split!"));
    timer::split();

    Ok(true)
}

pub fn split(key: &String, completed_splits: &mut HashSet<String>) -> bool {
    print_message(&format!("trying to split {key}"));
    let settings_map = asr::settings::Map::load();

    if completed_splits.contains(key) {
        print_message(&format!("-> {key} already split"));
        return false;
    }

    let value = settings_map.get(key);

    if value.is_none() {
        print_message(&format!("-> {key} not found"));
        return false;
    }

    let value = value.unwrap().get_bool();
    if value.is_none() {
        print_message(&format!("-> {key} not a bool"));
        return false;
    }

    if !value.unwrap() {
        print_message(&format!("-> {key} set to false"));
        return false;
    }

    if !completed_splits.insert(key.to_owned()) {
        print_message(&format!("-> {key} already split"));
        return false;
    }

    print_message(&format!("-> {key} split!"));
    timer::split();

    true
}
