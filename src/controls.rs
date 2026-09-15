//! Mapping from keys to game actions, independent of any windowing library.

/// The subset of keys the game cares about. Frontends translate their own key
/// codes into this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    H,
    J,
    K,
    L,
    Space,
    Backspace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    RotateCw,
    RotateCcw,
    RotateStop,
    MoveLeft,
    MoveRight,
    MoveStop,
    TrySpawn,
    Reset,
}

pub fn on_press(key: Key) -> Option<Action> {
    use Key::*;
    Some(match key {
        Up | K => Action::RotateCw,
        Down | J => Action::RotateCcw,
        Left | H => Action::MoveLeft,
        Right | L => Action::MoveRight,
        Space => Action::TrySpawn,
        Backspace => Action::Reset,
    })
}

pub fn on_release(key: Key) -> Option<Action> {
    use Key::*;
    match key {
        Up | Down | K | J => Some(Action::RotateStop),
        Left | Right | H | L => Some(Action::MoveStop),
        Space | Backspace => None,
    }
}
