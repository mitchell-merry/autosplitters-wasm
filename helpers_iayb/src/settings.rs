use ron;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Deserialize)]
enum Setting {
    TitleSetting {
        key: String,
        description: String,
        tooltip: Option<String>,
        subsettings: Option<Vec<Setting>>,
    },
    BoolSetting {
        key: String,
        description: String,
        tooltip: Option<String>,
        default: Option<bool>,
    },
}

pub fn initialise_settings(ron_string: &str) -> Result<HashMap<String, bool>, Box<dyn Error>> {
    let mut settings_map: HashMap<String, bool> = HashMap::new();

    let settings = ron::from_str::<Vec<Setting>>(ron_string)?;

    for setting in settings.iter() {
        initialise_setting(setting, 0, &mut settings_map);
    }

    Ok(settings_map)
}

fn initialise_setting(
    setting: &Setting,
    heading_level: u32,
    settings_defaults_map: &mut HashMap<String, bool>,
) {
    match setting {
        Setting::TitleSetting {
            key,
            description,
            tooltip,
            subsettings,
        } => {
            asr::settings::gui::add_title(key, description, heading_level);

            if let Some(tooltip) = tooltip {
                asr::settings::gui::set_tooltip(key, tooltip);
            }

            if subsettings.is_none() {
                return;
            }

            for ss in subsettings.as_ref().unwrap() {
                initialise_setting(ss, heading_level + 1, settings_defaults_map);
            }
        }
        Setting::BoolSetting {
            key,
            description,
            tooltip,
            default,
        } => {
            let default_value = default.unwrap_or_default();
            asr::settings::gui::add_bool(key, description, default_value);

            if let Some(tooltip) = tooltip {
                asr::settings::gui::set_tooltip(key, tooltip);
            }

            settings_defaults_map.insert(String::from(key), default_value);
        }
    }
}
