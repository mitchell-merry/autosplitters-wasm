extern crate helpers;
mod classes;
mod memory;

use crate::classes::{Classes, SceneLoader};
use asr::future::retry;
use asr::game_engine::unity::mono::{Image, Module, Version};
use asr::timer::set_variable;
use asr::{future::next_tick, print_message, Process};
use helpers::error::SimpleError;
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
    let (module, image, classes) =
        helpers::try_load::wait_try_load_millis::<(Module, Image, Classes), _, _>(
            async || {
                print_message("  => loading module");
                let module = Module::attach(process, Version::V1Cattrs)
                    .ok_or(SimpleError::from("mono module not found"))?;
                print_message("  => module loaded, loading image");
                let image = module
                    .get_default_image(process)
                    .ok_or(SimpleError::from("default image not found"))?;
                print_message("  => image loaded, loading classes");
                let scene_loader_class = image
                    .get_class(process, &module, "SceneLoader")
                    .ok_or(SimpleError::from("class not found"))?;
                print_message("  => loaded the");

                Ok((
                    module,
                    image,
                    Classes {
                        scene_loader: scene_loader_class,
                    },
                ))
            },
            Duration::from_millis(500),
        )
        .await;

    let scene_loader = SceneLoader::bind(process, &module, &image).await;

    while process.is_open() {
        let sl_instance = classes
            .scene_loader
            // does not immediately exist
            .wait_get_static_instance(process, &module, "_instance")
            .await;
        let sl = scene_loader
            .read(process, sl_instance)
            .expect("should exist");

        set_variable("is loading", &format!("{}", !sl.is_loading));

        // Prepare for the next iteration
        // memory.invalidate();

        next_tick().await;
    }

    Ok(())
}
