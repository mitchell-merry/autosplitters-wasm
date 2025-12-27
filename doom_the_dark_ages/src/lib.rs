extern crate helpers;
use crate::player::IdPlayer;
use asr::signature::Signature;
use asr::{future::next_tick, PointerSize, Process};
use bytemuck::CheckedBitPattern;
use helpers::memory::scan_rel;
use helpers::watchers::pointer_path::PointerPath;
use helpers::watchers::Watcher;
use idtech::{IdTech, IdTechVersion};
use std::error::Error;

mod physics;
mod player;

asr::async_main!(stable);

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Hello, World!");

    loop {
        let process = Process::wait_attach("DOOMTheDarkAges.exe").await;
        process
            .until_closes(async {
                on_attach(&process).await.expect("problem");
            })
            .await;
    }
}

static GAME_SYSTEM_LOCAL_SIG: Signature<17> =
    Signature::new("FF 50 40 48 8D 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? 84 C0");

#[derive(CheckedBitPattern, Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
#[allow(dead_code)]
enum IdGameSystemLocalState {
    MainMenu = 0,
    Loading = 1,
    InGame = 2,
}

async fn on_attach(process: &Process) -> Result<(), Box<dyn Error>> {
    let idtech = helpers::try_load::wait_try_load::<IdTech, _, _>(async || {
        IdTech::try_load(process, IdTechVersion::IdTech8, "DOOMTheDarkAges.exe").await
    })
    .await;

    let mut memory = Memory::init(process, idtech)?;

    while process.is_open() {
        let state = &memory.state;
        if state.changed()? {
            asr::print_message(&format!(
                "state changed from {:?} to {:?}",
                state.old(),
                state.current()?
            ))
        }

        // let player = &memory.player;
        // let vel_x = player.velocity.x.current().unwrap_or(0f32);

        // Prepare for the next iteration
        memory.invalidate();

        next_tick().await;
    }

    Ok(())
}

struct Memory<'a> {
    state: Watcher<'a, IdGameSystemLocalState>,
    player: IdPlayer<'a>,
}

impl<'a> Memory<'a> {
    pub fn init(process: &'a Process, idtech: IdTech<'a>) -> Result<Self, Box<dyn Error>> {
        let game_system_local = scan_rel(
            &GAME_SYSTEM_LOCAL_SIG,
            process,
            "DOOMTheDarkAges.exe",
            0x6,
            0x4,
        )?;
        asr::print_message(&format!(
            "=> found idGameSystemLocal ptr at 0x{}",
            game_system_local
        ));

        // Get the classes we need - we assume that they exist by now,
        //   if they don't, it's a fatal error and we shouldn't retry

        Ok(Memory {
            state: PointerPath::new(
                process,
                game_system_local,
                PointerSize::Bit64,
                [idtech.get_offset("Game", "idGameSystemLocal", "state")?],
            )
            .into(),
            player: IdPlayer::init(
                &idtech,
                PointerPath::new(
                    process,
                    game_system_local,
                    PointerSize::Bit64,
                    [
                        idtech.get_offset("Game", "idGameSystemLocal", "mapInstance")?,
                        0x1988,
                        0xC0,
                        0x0,
                    ],
                ),
            )?,
        })
    }

    pub fn invalidate(&mut self) {
        self.state.invalidate();
        self.player.invalidate();
    }
}
