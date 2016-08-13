use na::Vector2;

pub const GRAVITY: Vector2<f32> = Vector2 { x: 0.0, y: -20.0 };

pub const WALL_RESTITUTION: f32 = 0.4;
pub const WALL_FRICTION: f32 = 0.4;

pub const POLYOMINO_DENSITY: f32 = 0.05;
pub const POLYOMINO_RESTITUTION: f32 = 0.3;
pub const POLYOMINO_FRICTION: f32 = 0.25;

pub const POLYOMINO_FORCE: f32 = polyominos::POLYOMINO_FORCE;
pub const POLYOMINO_ANG_FORCE: f32 = polyominos::POLYOMINO_ANG_FORCE;

pub const LEFT: f32 = 0.0;
pub const RIGHT: f32 = 12.0;
pub const BOTTOM: f32 = 0.0;
pub const TOP: f32 = 16.0;

pub const LINE_THRESHOLD: f32 = 0.05;
pub const BLOCKS_PER_LINE: usize = 12;

pub const BLOCK_DIST: f32 = 0.995 * (RIGHT - LEFT) / BLOCKS_PER_LINE as f32;
pub const BLOCK_SIZE: f32 = 0.96 * BLOCK_DIST;
pub const CORNER_RADIUS: f32 = 0.16 * BLOCK_SIZE;
pub const EDGES_PER_CORNER: u32 = 3;
pub const VERTS_PER_BLOCK: u32 = 4 * (EDGES_PER_CORNER + 1);

pub const SPAWN_DELAY_MS: u64 = 750;

pub use self::polyominos::POLYOMINOS;

mod polyominos {
    #[cfg(feature="tetrominos")]
    pub use self::tetrominos::*;
    #[cfg(feature="tetrominos")]
    mod tetrominos {
        pub const POLYOMINO_FORCE: f32 = 0.16;
        pub const POLYOMINO_ANG_FORCE: f32 = 0.14;
        pub const POLYOMINOS: [[[usize; 2]; 4]; 7] = [I, O, T, J, L, S, Z];
        const I: [[usize; 2]; 4] = [[0, 0], [1, 0], [2, 0], [3, 0]];
        const O: [[usize; 2]; 4] = [[0, 0], [0, 1], [1, 0], [1, 1]];
        const T: [[usize; 2]; 4] = [[0, 0], [1, 0], [2, 0], [1, 1]];
        const J: [[usize; 2]; 4] = [[0, 0], [1, 0], [2, 0], [2, 1]];
        const L: [[usize; 2]; 4] = [[0, 0], [1, 0], [2, 0], [0, 1]];
        const S: [[usize; 2]; 4] = [[1, 0], [2, 0], [0, 1], [1, 1]];
        const Z: [[usize; 2]; 4] = [[0, 0], [1, 0], [1, 1], [2, 1]];
    }

    #[cfg(feature="pentominos")]
    pub use self::pentominos::*;
    #[cfg(feature="pentominos")]
    mod pentominos {
        pub const POLYOMINO_FORCE: f32 = 0.03;
        pub const POLYOMINO_ANG_FORCE: f32 = 0.024;
        pub const POLYOMINOS: [[[usize; 2]; 5]; 18] = [
            [[0, 0], [0, 1], [0, 2], [0, 3], [0, 4]],
            [[0, 0], [0, 1], [0, 2], [0, 3], [1, 0]],
            [[0, 0], [0, 1], [0, 2], [0, 3], [1, 1]],
            [[0, 0], [0, 1], [0, 2], [0, 3], [1, 2]],
            [[0, 0], [0, 1], [0, 2], [0, 3], [1, 3]],
            [[0, 0], [0, 1], [0, 2], [1, 0], [1, 1]],
            [[0, 0], [0, 1], [0, 2], [1, 0], [1, 2]],
            [[0, 0], [0, 1], [0, 2], [1, 0], [2, 0]],
            [[0, 0], [0, 1], [0, 2], [1, 1], [1, 2]],
            [[0, 0], [0, 1], [0, 2], [1, 1], [2, 1]],
            [[0, 0], [0, 1], [0, 2], [1, 2], [1, 3]],
            [[0, 0], [0, 1], [1, 1], [1, 2], [2, 1]],
            [[0, 0], [0, 1], [1, 1], [1, 2], [2, 2]],
            [[0, 0], [0, 1], [1, 1], [2, 1], [2, 2]],
            [[0, 0], [1, 0], [1, 1], [1, 2], [2, 1]],
            [[0, 0], [1, 0], [1, 1], [1, 2], [2, 2]],
            [[0, 0], [1, 0], [1, 1], [2, 1], [3, 1]],
            [[0, 1], [1, 0], [1, 1], [1, 2], [2, 1]],
        ];
    }

}
