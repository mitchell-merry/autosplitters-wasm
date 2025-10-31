use helpers::pointer::{Invalidatable, MemoryWatcher, UnityImage, UnityPointerPath};

pub struct Memory<'a> {
    pub combat_time: MemoryWatcher<'a, UnityPointerPath<'a>, f32>,
}

impl<'a> Memory<'a> {
    pub fn new(unity: UnityImage<'a>) -> Memory<'a> {
        Memory {
            combat_time: MemoryWatcher::from(unity.path(
                "GameManager",
                0,
                &["instance", "levelController", "combatTimer", "timer"],
            )),
        }
    }

    pub fn invalidate(&mut self) {
        self.combat_time.invalidate();
    }
}
