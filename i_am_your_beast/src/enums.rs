use bytemuck::CheckedBitPattern;

// these names come from code directly

// Timer pauses 0 and 1
#[derive(CheckedBitPattern, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
#[allow(dead_code, non_camel_case_types)]
pub enum SceneTransitionState {
    TransitioningOut = 0,
    Holding = 1,
    TransitioningIn = 2,

    #[default]
    Unknown = 67,
}

#[derive(CheckedBitPattern, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
#[allow(dead_code, non_camel_case_types)]
pub enum LevelState {
    Intro = 0,
    Active = 1,
    Completed = 2,
    Failed = 3,

    #[default]
    Unknown = 67,
}
