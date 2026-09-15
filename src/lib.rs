//! Frustration Tetris: a Tetris clone where every piece is a rigid body.
//!
//! This crate contains the renderer-agnostic parts of the game: the physics
//! simulation and rules ([`game`]), the key-to-action mapping ([`controls`]),
//! block geometry shared by all renderers ([`mesh`]) and CPU text
//! rasterisation for the score display ([`text`]). A frontend only has to
//! forward key presses to [`game::Game::execute`], call
//! [`game::Game::advance`] once per frame and draw whatever
//! [`game::Game::blocks`] yields.

pub mod consts;
pub mod controls;
pub mod game;
pub mod mesh;
pub mod render;
pub mod text;
pub mod view;

pub use controls::{Action, Key};
pub use game::{BlockInstance, Game};
