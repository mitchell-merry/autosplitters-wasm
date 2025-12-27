extern crate helpers;
mod memory;
mod settings;

use crate::memory::Memory;
use crate::settings::Settings;
use asr::future::retry;
use asr::game_engine::unity::mono::{Image, Module};
use asr::settings::Gui;
use asr::timer::set_variable;
use asr::{future::next_tick, print_message, Process};
use helpers::error::SimpleError;
use helpers::watchers::unity::UnityImage;
use std::error::Error;
use std::rc::Rc;

asr::async_main!(stable);

const PROCESS_NAMES: [&str; 1] = [
    // Windows
    "I Am Your Beast.exe",
];

#[derive(Default)]
struct MeasuredState {}

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

            let gm = image.get_class(process, &module, "GameManager");
            print_message(&format!("  => gm found {}", gm.is_some()));

            Ok((module, image))
        },
        std::time::Duration::from_millis(500),
    )
    .await;

    let unity = UnityImage::new(process, Rc::new(module), image);
    let mut memory = Memory::new(unity);
    let mut measured_state = MeasuredState::default();

    while process.is_open() {
        settings.update();

        next_tick().await;

        memory.invalidate();

        if let Err(err) = tick(process, &memory, &mut measured_state, settings).await {
            print_message(&format!("tick failed: {err}"));
        }
    }

    Ok(())
}

async fn tick<'a>(
    _process: &'a Process,
    memory: &Memory<'a>,
    _measured_state: &mut MeasuredState,
    _settings: &mut Settings,
) -> Result<(), Box<dyn Error>> {
    set_variable("combat time", &format!("{}", memory.combat_time.current()?));

    Ok(())
}
