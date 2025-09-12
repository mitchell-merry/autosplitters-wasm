extern crate helpers;
mod enums;
mod memory;
mod settings;

use crate::memory::Memory;
use crate::settings::{LevelCompleteSetting, Settings};
use asr::future::retry;
use asr::game_engine::unity::mono::{Image, Module};
use asr::settings::Gui;
use asr::timer::{
    pause_game_time, reset, resume_game_time, set_game_time, set_variable, split, start, state,
    TimerState,
};
use asr::{future::next_tick, print_message, Process};
use helpers::error::SimpleError;
use helpers::pointer::{Invalidatable, Readable2, UnityImage};
use std::error::Error;

asr::async_main!(stable);

const PROCESS_NAMES: [&str; 2] = [
    // Windows
    "Cuphead.exe",
    // Mac
    "Cuphead",
];

const SCENE_CUTSCENE_INTRO: &str = "scene_cutscene_intro";

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        print_message(&panic_info.to_string());
    }));

    print_message("Hello, World!");

    let mut settings = Settings::register();

    loop {
        let process = retry(|| PROCESS_NAMES.iter().find_map(|name| Process::attach(name))).await;

        process
            .until_closes(async {
                let res = on_attach(&process, &mut settings).await;
                if let Err(err) = res {
                    print_message(&format!("error occuring on_attach: {}", err));
                } else {
                    print_message("detached from process");
                }
            })
            .await;
    }
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Box<dyn Error>> {
    let (module, image) = helpers::try_load::wait_try_load_millis::<(Module, Image), _, _>(
        async || {
            print_message("  => loading module");
            let module = Module::attach_auto_detect(process)
                .ok_or(SimpleError::from("mono module not found"))?;
            print_message("  => module loaded, loading image");
            let image = module
                .get_default_image(process)
                .ok_or(SimpleError::from("default image not found"))?;
            print_message("  => image loaded, loading classes");

            Ok((module, image))
        },
        std::time::Duration::from_millis(500),
    )
    .await;

    let unity = UnityImage::new(process, &module, &image);
    let mut memory = Memory::new(unity);

    while process.is_open() {
        settings.update();

        next_tick().await;

        memory.invalidate();

        if let Err(err) = tick(&memory, settings).await {
            // print_message(&format!("tick failed: {err}"));
        }
    }

    Ok(())
}

async fn tick<'a>(memory: &Memory<'a>, settings: &mut Settings) -> Result<(), Box<dyn Error>> {
    set_variable(
        "done loading scene async",
        &format!("{}", memory.done_loading.current()?),
    );
    set_variable(
        "scene loader instance",
        &format!("0x{}", memory.scene_loader_instance.current()?),
    );
    set_variable(
        "currently loading",
        &format!("{}", memory.currently_loading.current()?),
    );
    let scene = String::from_utf16(memory.scene.current()?.as_slice())?;
    set_variable("scene name", &format!("{}", scene));
    print_message("b");
    let previous_scene = String::from_utf16(memory.previous_scene.current()?.as_slice())?;
    set_variable("previous scene name", &format!("{}", previous_scene));

    set_variable("in game", &format!("{}", memory.in_game.current()?));
    set_variable("current level", &format!("{:?}", memory.level.current()?));
    set_variable("level won", &format!("{}", memory.level_won.current()?));
    set_variable(
        "level ending",
        &format!("{}", memory.level_ending.current()?),
    );
    set_variable(
        "level time (raw)",
        &format!(
            "{}",
            f32::trunc(memory.level_time.current()? * 100.0) / 100.0
        ),
    );
    set_variable(
        "level time",
        &format!("{:.2}", memory.level_time.current()?),
    );
    set_variable("lsd time (raw)", &format!("{}", memory.lsd_time.current()?));
    set_variable(
        "kd is first retry",
        &format!("{}", memory.kd_is_first_entry.current()?),
    );
    set_variable(
        "is dice palace",
        &format!("{}", memory.level_is_dice.current()?),
    );
    set_variable(
        "is dice palace main",
        &format!("{}", memory.level_is_dice_main.current()?),
    );
    set_variable(
        "save file index",
        &format!("{}", memory.save_file_index.current()?),
    );
    set_variable("save files", &format!("{}", memory.save_files.current()?));

    if state() == TimerState::NotRunning
        && ((scene == SCENE_CUTSCENE_INTRO
        && memory.in_game.current()?
        // just started loading
        && !memory.done_loading.current()?
        && memory.done_loading.old().is_some_and(|l| l))
            || (settings.individual_level_mode
                && memory.level_time.old().is_some_and(|t| t == 0f32)
                && memory.level_time.current()? > 0f32
                && (!memory.level_is_dice.current()? || memory.lsd_time.current()? == 0f32)))
    {
        start();
    }

    if state() == TimerState::Running {
        if memory.done_loading.changed()? {
            print_message("  => done loading changed");
        }

        if settings.individual_level_mode {
            pause_game_time();

            let time = if memory.level_won.current()? {
                memory.lsd_time.current()?
            } else {
                memory.level_time.current()? + memory.lsd_time.current()?
            };

            set_game_time(asr::time::Duration::seconds_f32(time));
        } else if memory.is_loading()? {
            pause_game_time();
        } else {
            resume_game_time();
        }

        let level = memory.level.current()?;
        let should_split = if let Some(target_scenes) = level.split_on_scene_transition_to() {
            // split if the level transitions out to another specific scene (e.g. tutorial)
            memory.scene.changed()? && target_scenes.contains(scene.as_str())
        } else if settings.split_level_complete == LevelCompleteSetting::OnKnockout
            || settings.individual_level_mode
            || level.always_split_on_knockout()
        {
            // split on knockout
            level.get_type().is_split_enabled(settings)
                && memory.level_won.old().is_some_and(|w| !w)
                && memory.level_won.current()?
        } else {
            // split after scoreboard
            let previous_scene = String::from_utf16(memory.previous_scene.current()?.as_slice())?;
            let previous_scene = previous_scene.as_str();

            // split when we start loading, this gives cleaner splits (segment timer is at 0.00 in
            //   the loading screen)
            level.get_type().is_split_enabled(settings)
                && previous_scene == "scene_win"
                && memory.done_loading.changed()?
                && memory.is_loading()?
        };

        if should_split {
            split();
        }

        let should_reset = if settings.individual_level_mode {
            if memory.level_is_dice.current()? {
                memory.kd_is_first_entry.old().is_some_and(|f| !f)
                    && memory.kd_is_first_entry.current()?
            } else {
                memory.level_time.old().is_some_and(|t| t > 0f32)
                    && memory.level_time.current()? == 0f32
            }
        } else {
            false
        };

        if should_reset {
            reset();
        }
    }

    Ok(())
}
