extern crate helpers;
mod classes;
mod memory;

use crate::memory::Memory;
use asr::future::retry;
use asr::game_engine::unity::mono::{Image, Module, Version};
use asr::timer::{pause_game_time, resume_game_time, set_variable};
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
            print_message(&format!("tick failed: {err}"));
        }
    }

    Ok(())
}

async fn tick<'a>(memory: &Memory<'a>) -> Result<(), Box<dyn Error>> {
    set_variable(
        "is loading",
        &format!("{}", !memory.done_loading.current()?),
    );
    set_variable("in game", &format!("{}", memory.in_game.current()?));
    set_variable(
        "scene name",
        &format!("{}", memory.scene.current()?.validate_utf8()?),
    );

    if !memory.done_loading.current()? {
        pause_game_time();
    } else {
        resume_game_time();
    }

    Ok(())
}
