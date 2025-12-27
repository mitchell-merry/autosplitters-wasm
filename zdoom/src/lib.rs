use std::time::Duration;
use std::{collections::HashMap, rc::Rc};

use asr::{print_message, signature::Signature, Address, Error, Process};
use bytemuck::CheckedBitPattern;
use once_cell::unsync::OnceCell;

use self::{
    level::Level, name_manager::NameManager, pclass::PClass, player::Player, tarray::TArray,
};

pub mod level;
pub mod name_manager;
pub mod pclass;
pub mod player;
pub mod tarray;

pub struct ZDoom<'a> {
    process: &'a Process,
    pub memory: Rc<Memory>,
    pub name_data: Rc<NameManager<'a>>,
    pub classes: OnceCell<HashMap<String, PClass<'a>>>,
    _actor_class: OnceCell<PClass<'a>>,

    pub level: Level<'a>,
    pub player: OnceCell<Player<'a>>,
    pub gameaction: OnceCell<GameAction>,
}

impl<'a> ZDoom<'a> {
    pub async fn wait_try_load<T, F>(
        process: &'a Process,
        version: ZDoomVersion,
        main_module_name: &str,
        load_fn: F,
    ) -> (ZDoom<'a>, T)
    where
        F: Fn(&HashMap<String, PClass<'a>>) -> Result<T, Option<Error>>,
    {
        asr::print_message(&format!("zdoom: Using version {version:?}"));
        let cooldown = Duration::from_secs(3);

        let fail_action = || async {
            print_message(&format!(
                "try_load unsuccessful, waiting {}s...",
                cooldown.as_secs()
            ));
            asr::future::sleep(cooldown).await;
        };

        loop {
            let memory = Memory::new(process, version, main_module_name);
            if memory.is_err() {
                fail_action().await;
                continue;
            }

            let memory = memory.unwrap();

            let memory = Rc::new(memory);
            let name_data = Rc::new(NameManager::new(process, memory.namedata_addr));
            let level = Level::new(
                process,
                memory.clone(),
                name_data.clone(),
                memory.level_addr,
            );

            let zdoom = ZDoom {
                process,
                memory,
                name_data,
                level,
                classes: OnceCell::new(),
                _actor_class: OnceCell::new(),
                player: OnceCell::new(),
                gameaction: OnceCell::new(),
            };

            let classes = zdoom.classes();
            // assert that we have the Actor class, we need it for Player shenanigans
            if classes.is_err() {
                print_message("try_load: error loading classes");
                fail_action().await;
                continue;
            }

            let classes = classes.unwrap();
            if !classes.contains_key("Actor") {
                print_message("try_load: missing Actor class");
                fail_action().await;
                continue;
            }

            let result = load_fn(classes);

            if result.is_err() {
                print_message("try_load: error running load_fn");
                fail_action().await;
                continue;
            }

            print_message("try_load successful!");
            return (zdoom, result.unwrap());
        }
    }

    pub fn classes(&self) -> Result<&HashMap<String, PClass<'a>>, Error> {
        self.classes.get_or_try_init(|| {
            let mut classes: HashMap<String, PClass<'a>> = HashMap::new();
            let all_classes = TArray::new(self.process, self.memory.all_classes_addr);

            for class in all_classes.iter::<u64>()? {
                let pclass = PClass::<'a>::new(
                    self.process,
                    self.memory.clone(),
                    self.name_data.clone(),
                    class.into(),
                );
                let name = pclass.name()?.to_owned();

                classes.insert(name, pclass);
            }

            Ok(classes)
        })
    }

    pub fn invalidate_cache(&mut self) -> Result<(), Error> {
        self.level.invalidate_cache();
        self.player = OnceCell::new();
        self.gameaction = OnceCell::new();

        Ok(())
    }

    pub fn find_class(&self, name: &str) -> Result<Option<&PClass<'a>>, Error> {
        Ok(self.classes()?.get(name))
    }

    pub fn dump(&self) -> Result<(), Error> {
        print_message(
            r"#include <cstdint>

template <class T>
class TArray
{
    T* Array;
	unsigned int Count;
	unsigned int Most;
};
",
        );
        for (name, class) in self.classes()?.iter() {
            let c = class
                .show_class()
                .unwrap_or(format!("// failed getting {name}"));

            print_message(&format!("{c}\n"));
        }

        Ok(())
    }

    pub fn player<'b>(&'b self) -> Result<&'b Player<'a>, Option<Error>> {
        self.player.get_or_try_init(|| {
            let actor_class = self
                .classes()?
                .get("Actor")
                .expect("we should have asserted the Actor class was found")
                .to_owned();

            Ok(Player::new(
                self.process,
                self.memory.clone(),
                // 0x0 is the first index
                self.memory.players_addr + 0x0,
                actor_class,
            ))
        })
    }

    pub fn gameaction(&self) -> Result<GameAction, Error> {
        self.gameaction
            .get_or_try_init(|| self.process.read(self.memory.gameaction_addr))
            .map(|v| v.to_owned())
    }
}

// disclaimer: I don't know much about the different zdoom versions work...
// i have only tried this with a few games
#[derive(Clone, Copy, Debug)]
pub enum ZDoomVersion {
    Lzdoom3_82,   // Dismantled: Director's Cut
    Gzdoom4_8Pre, // Selaco
    Gzdoom4_8_2,  // Snap the Sentinel
}

type ScanFn = fn(process: &Process, module_range: (Address, u64)) -> Result<Address, Option<Error>>;

fn find_addr_or_panic(
    name: &str,
    process: &Process,
    module_range: (Address, u64),
    sigs: Vec<ScanFn>,
) -> Address {
    for (i, sig) in sigs.iter().enumerate() {
        if let Ok(addr) = sig(process, module_range) {
            asr::print_message(&format!(
                "Found {name} at 0x{addr} with signature index {i}"
            ));
            return addr;
        }
    }

    panic!("unable to find addr for {name}");
}

fn scan<const N: usize>(
    signature: Signature<N>,
    process: &Process,
    (addr, len): (Address, u64),
    offset: u32,
    next_instruction: u32,
) -> Result<Address, Option<Error>> {
    let addr = signature
        .scan_process_range(process, (addr, len))
        .ok_or(None)?
        + offset;

    Ok(addr + process.read::<u32>(addr)? + next_instruction)
}

pub struct Memory {
    namedata_addr: Address,
    players_addr: Address,
    all_classes_addr: Address,
    level_addr: Address,
    gameaction_addr: Address,

    offsets: Offsets,
}

impl Memory {
    fn new(
        process: &Process,
        version: ZDoomVersion,
        main_module_name: &str,
    ) -> Result<Memory, Error> {
        let module_range = process.get_module_range(main_module_name)?;

        let namedata_sigs: Vec<ScanFn> = vec![
            |p, mr| {
                scan(
                    Signature::<19>::new(
                        "0F 84 ?? ?? ?? ?? 48 8B D1 41 B0 01 48 8D 0D ?? ?? ?? ??",
                    ),
                    p,
                    mr,
                    0xF,
                    0x4,
                )
            },
            |p, mr| {
                scan(
                    Signature::<23>::new(
                        "45 33 C0 48 8B D6 48 8D 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? 44 8B C0 8B 15",
                    ),
                    p,
                    mr,
                    0x9,
                    0x4,
                )
            },
        ];

        let players_sigs: Vec<ScanFn> = vec![|p, mr| {
            scan(
                Signature::<18>::new("48 8D 05 ?? ?? ?? ?? 48 03 C8 E8 ?? ?? ?? ?? 48 63 05"),
                p,
                mr,
                0x3,
                0x4,
            )
        }];

        let all_classes_sigs: Vec<ScanFn> = vec![
            |p, mr| {
                scan(
                    Signature::<22>::new(
                        "48 8B 1D ?? ?? ?? ?? 8B 05 ?? ?? ?? ?? 48 8D 3C C3 48 3B DF 0F 84",
                    ),
                    p,
                    mr,
                    0x3,
                    0x4,
                )
            },
            |p, mr| {
                scan(Signature::<26>::new(
                    "49 89 46 30 48 8B 1D ?? ?? ?? ?? 8B 05 ?? ?? ?? ?? 48 8D 3C C3 48 3B DF 0F 84",
                ), p, mr, 0x7, 0x4)
            },
        ];

        let level_sigs: Vec<ScanFn> = vec![
            |p, mr| {
                scan(
                    Signature::<13>::new("75 D1 89 2D ?? ?? ?? ?? 8B 05 ?? ?? ??"),
                    p,
                    mr,
                    0x4,
                    0x4,
                )
            },
            |p, mr| {
                let a = p.read::<u64>(scan(
                    Signature::<13>::new("48 8B 05 ?? ?? ?? ?? 48 39 03 75 09 E8"),
                    p,
                    mr,
                    0x3,
                    0x4,
                )?);

                Ok(a?.into())
            },
        ];

        let gameaction_sigs: Vec<ScanFn> = vec![|p, mr| {
            scan(
                Signature::<33>::new("B2 01 89 05 ?? ?? ?? ?? E8 ?? ?? ?? ?? C7 05 ?? ?? ?? ?? 03 00 00 00 C7 05 ?? ?? ?? ?? 02 00 00 00"),
                p, mr, 0xF, 0x8
            )
        }];

        Ok(Memory {
            namedata_addr: find_addr_or_panic("namedata", process, module_range, namedata_sigs),
            players_addr: find_addr_or_panic("players", process, module_range, players_sigs),
            all_classes_addr: find_addr_or_panic(
                "all_classes",
                process,
                module_range,
                all_classes_sigs,
            ),
            level_addr: find_addr_or_panic("level", process, module_range, level_sigs),
            gameaction_addr: find_addr_or_panic(
                "gameaction",
                process,
                module_range,
                gameaction_sigs,
            ),
            offsets: Offsets::new(version),
        })
    }
}

struct Offsets {
    pclass_fields: u64,
    level_mapname: u64,
    level_sectors: u64,
    sector_thinglist: u64,
}

impl Offsets {
    fn new(version: ZDoomVersion) -> Self {
        match version {
            ZDoomVersion::Lzdoom3_82 => Self {
                pclass_fields: 0x78,
                level_mapname: 0x2C0,
                level_sectors: 0x10,
                sector_thinglist: 0x180,
            },
            ZDoomVersion::Gzdoom4_8Pre => Self {
                pclass_fields: 0x80,
                level_mapname: 0x9F8,
                level_sectors: 0x50,
                sector_thinglist: 0x268,
            },
            ZDoomVersion::Gzdoom4_8_2 => Self {
                pclass_fields: 0x78,
                level_mapname: 0x9D8,
                level_sectors: 0x50,
                sector_thinglist: 0x268,
            },
        }
    }
}

#[derive(CheckedBitPattern, Clone, Copy, Debug, Default, PartialEq)]
#[repr(u32)]
pub enum GameAction {
    #[default]
    Nothing,
    LoadLevel, // not used.
    NewGame,
    NewGame2,
    RecordGame,
    LoadGame,
    LoadGameHideCon,
    LoadGamePlayDemo,
    AutoLoadGame,
    SaveGame,
    AutoSave,
    PlayDemo,
    Completed,
    Slideshow,
    WorldDone,
    Screenshot,
    ToggleMap,
    FullConsole,
    ResumeConversation,
    Intro,
    Intermission,
    TitleLoop,
}
