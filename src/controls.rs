use glium::glutin::VirtualKeyCode;
use glium::glutin::VirtualKeyCode::*;

pub struct Controls;

pub enum Action {
    RotateCW, RotateCCW, RotateStop,
    MoveLeft, MoveRight, MoveStop,
    TrySpawn,
    GameReset,
}

impl Controls {
    pub fn resolve_press(&self, key: VirtualKeyCode) -> Option<Action> {
        Some(match key {
            Up | K    => Action::RotateCW,
            Down | J  => Action::RotateCCW,
            Left | H  => Action::MoveLeft,
            Right | L => Action::MoveRight,
            Space     => Action::TrySpawn,
            Back      => Action::GameReset,
            _         => return None
        })
    }

    pub fn resolve_release(&self, key: VirtualKeyCode) -> Option<Action> {
        Some(match key {
            Up | Down | K | J    => Action::RotateStop,
            Left | H | Right | L => Action::MoveStop,
            _                    => return None,
        })
    }
}
