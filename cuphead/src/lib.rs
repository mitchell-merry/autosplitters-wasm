extern crate helpers;
mod enums;
mod memory;

use crate::enums::Levels;
use crate::memory::Memory;
use asr::future::retry;
use asr::game_engine::unity::mono::{Image, Module, Version};
use asr::timer::{
    pause_game_time, resume_game_time, set_variable, split, start, state, TimerState,
};
use asr::{future::next_tick, print_message, Process};
use helpers::error::SimpleError;
use helpers::pointer::{Invalidatable, Readable2, UnityImage};
use std::error::Error;
use std::time::Duration;

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

    loop {
        let process = retry(|| PROCESS_NAMES.iter().find_map(|name| Process::attach(name))).await;

        process
            .until_closes(async {
                let res = on_attach(&process).await;
                if let Err(err) = res {
                    print_message(&format!("error occuring on_attach: {}", err));
                } else {
                    print_message("detached from process");
                }
            })
            .await;
    }
}

async fn on_attach(process: &Process) -> Result<(), Box<dyn Error>> {
    let (module, image) = helpers::try_load::wait_try_load_millis::<(Module, Image), _, _>(
        async || {
            print_message("  => loading module");
            let module = Module::attach(process, Version::V1Cattrs)
                .ok_or(SimpleError::from("mono module not found"))?;
            print_message("  => module loaded, loading image");
            let image = module
                .get_default_image(process)
                .ok_or(SimpleError::from("default image not found"))?;
            print_message("  => image loaded, loading classes");

            Ok((module, image))
        },
        Duration::from_millis(500),
    )
    .await;

    let unity = UnityImage::new(process, &module, &image);
    let mut memory = Memory::new(unity);

    while process.is_open() {
        next_tick().await;
        memory.invalidate();

        if let Err(err) = tick(&memory).await {
            // print_message(&format!("tick failed: {err}"));
        }
    }

    Ok(())
}

async fn tick<'a>(memory: &Memory<'a>) -> Result<(), Box<dyn Error>> {
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
        "save file index",
        &format!("{}", memory.save_file_index.current()?),
    );
    set_variable("save files", &format!("{}", memory.save_files.current()?));

    // TODO: individual level mode
    if state() == TimerState::NotRunning
        && scene == SCENE_CUTSCENE_INTRO
        && memory.in_game.current()?
        // just started loading
        && !memory.done_loading.current()?
        && memory.done_loading.old().is_some_and(|l| l)
    {
        start();
    }

    if state() == TimerState::Running {
        if memory.done_loading.changed()? {
            print_message("  => done loading changed");
        }

        if memory.is_loading()? {
            pause_game_time();
        } else {
            resume_game_time();
        }

        // TODO: setting to always split on knockout instead of after scoreboard
        // TODO: individual level mode
        let should_split = if memory.level.current()? == Levels::Devil {
            // split on knockout
            memory.level_won.old().is_some_and(|w| !w) && memory.level_won.current()?
        } else {
            // split after scoreboard
            let previous_scene = String::from_utf16(memory.previous_scene.current()?.as_slice())?;
            let previous_scene = previous_scene.as_str();

            // split when we start loading, this gives cleaner splits (segment timer is at 0.00 in
            //   the loading screen)
            previous_scene == "scene_win"
                && memory.done_loading.changed()?
                && memory.is_loading()?
        };

        if should_split {
            split()
        }
    }

    Ok(())
}
