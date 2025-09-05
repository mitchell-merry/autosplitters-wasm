extern crate helpers;
mod classes;
mod memory;

use asr::future::retry;
use asr::game_engine::unity::mono::{Image, Module, Version};
use asr::timer::{pause_game_time, resume_game_time, set_variable};
use asr::{future::next_tick, print_message, Process};
use helpers::error::SimpleError;
use helpers::pointer::{Invalidatable, MemoryWatcher, Readable2, UnityImage};
use std::error::Error;
use std::time::Duration;

asr::async_main!(stable);

const PROCESS_NAMES: [&str; 3] = [
    // Windows
    "Cuphead.exe",
    // Mac
    "Cuphead",
    "Hollow Knight", // testing lol
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
                on_attach(&process).await.expect("problem? trollface");
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
    let path = unity.path::<2>("SceneLoader", 0, &["_instance", "doneLoadingSceneAsync"]);
    let mut done_loading: MemoryWatcher<_, bool> = MemoryWatcher::from(path); //.default_given(true);

    while process.is_open() {
        // let sl_instance = classes
        //     .scene_loader
        //     // does not immediately exist
        //     .wait_get_static_instance(process, &module, "_instance")
        //     .await;
        // let sl = scene_loader
        //     .read(process, sl_instance)
        //     .expect("should exist");

        set_variable("is loading", &format!("{}", !done_loading.current()?));

        if !done_loading.current()? {
            pause_game_time();
        } else {
            resume_game_time();
        }

        // Prepare for the next iteration
        done_loading.invalidate();

        next_tick().await;
    }

    Ok(())
}
