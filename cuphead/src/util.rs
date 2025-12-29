use std::error::Error;
use crate::memory::Memory;
use crate::settings::Settings;
use crate::enums::Levels;
use crate::enums::Grade;
use crate::enums::Mode;
use asr::{print_message};

pub fn format_seconds(secs: f32) -> String {
    let hours = (secs / 3600.0).floor() as u64;
    let minutes = ((secs % 3600.0) / 60.0).floor() as u64;
    let seconds = ((secs % 60.0) * 100.0).trunc() / 100.0; // truncate instead of round

    if hours > 0 {
        // HH:MM:SS.MM
        format!("{hours}:{minutes:02}:{seconds:05.2}")
    } else if minutes > 0 {
        // MM:SS.MM
        format!("{minutes}:{seconds:05.2}")
    } else {
        // SS.MM
        format!("{seconds:.2}")
    }
}


