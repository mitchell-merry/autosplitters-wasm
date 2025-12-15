extern crate helpers;

use asr::future::retry;
use asr::timer::{pause_game_time, resume_game_time, start, state, TimerState};
use asr::{future::next_tick, print_message, set_tick_rate, Process};
use helpers::error::SimpleError;
use std::error::Error;

asr::async_main!(stable);

const PROCESS_NAMES: [&str; 2] = [
    // Windows
    "autosplitting_test_project.exe",
    // Mac
    "Cuphead",
];

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        print_message(&panic_info.to_string());
    }));

    print_message("Hello, World!");
    pause_game_time();
    set_tick_rate(120.0);

    loop {
        let process = retry(|| PROCESS_NAMES.iter().find_map(|name| Process::attach(name))).await;

        process
            .until_closes(async {
                let res = on_attach(&process).await;
                if let Err(err) = res {
                    print_message(&format!("error occurring on_attach: {}", err));
                } else {
                    print_message("detached from process");
                }
            })
            .await;
    }
}

async fn on_attach(process: &Process) -> Result<(), Box<dyn Error>> {
    let (addr, _) = process
        .get_module_range("autosplitting_test_project.exe")
        .map_err(|_| SimpleError::from("failed to get module range of main module"))?;

    let addr = addr + 0x4000;
    asr::timer::set_variable("addr", &format!("{}", addr));

    pause_game_time();
    start();

    let mut tick_count = 0;

    while process.is_open() {
        tick_count = tick_count + 1;

        let x: bool = process
            .read(addr)
            .map_err(|_| SimpleError::from("failed to read address"))?;
        asr::timer::set_variable("the bool", &format!("{}", x));
        asr::timer::set_variable("tick count", &format!("{}", tick_count));

        next_tick().await;

        if state() == TimerState::Running {
            if x {
                pause_game_time();
            } else {
                resume_game_time();
            }
        }
    }

    Ok(())
}
