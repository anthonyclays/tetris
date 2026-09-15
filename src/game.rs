//! Game rules and physics simulation, independent of any renderer.

use std::time::Duration;

use rapier2d::prelude::*;

use crate::consts::*;
use crate::controls::Action;

/// Everything a renderer needs to draw one block.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockInstance {
    /// World-space centre of the block.
    pub position: [f32; 2],
    /// Rotation in radians, counter-clockwise.
    pub angle: f32,
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotate {
    Clockwise,
    Counterclockwise,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shift {
    Left,
    Right,
}

/// One rigid body made of several block colliders.
struct Piece {
    body: RigidBodyHandle,
    color: [f32; 3],
}

/// A completed row: blocks whose centre lies within `half_height` of
/// `center` belong to it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Line {
    center: f32,
    half_height: f32,
}

pub struct Game {
    world: PhysicsWorld,
    pieces: Vec<Piece>,
    /// The piece the player currently steers, if any.
    controlled: Option<RigidBodyHandle>,
    rotate: Option<Rotate>,
    shift: Option<Shift>,
    score: usize,
    /// Simulated time in seconds.
    time: f32,
    last_spawn: Option<f32>,
    /// Real time not yet consumed by fixed simulation steps.
    accumulator: f32,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Game {
            world: create_world(),
            pieces: Vec::new(),
            controlled: None,
            rotate: None,
            shift: None,
            score: 0,
            time: 0.0,
            last_spawn: None,
            accumulator: 0.0,
        }
    }

    pub fn score(&self) -> usize {
        self.score
    }

    /// Simulated time since the game started, in seconds.
    pub fn time(&self) -> f32 {
        self.time
    }

    /// Whether the player is currently steering a piece.
    pub fn has_controlled_piece(&self) -> bool {
        self.controlled.is_some()
    }

    /// All blocks currently in the playfield, ready for drawing.
    pub fn blocks(&self) -> impl Iterator<Item = BlockInstance> + '_ {
        self.pieces.iter().flat_map(move |piece| {
            let body = &self.world.bodies[piece.body];
            body.colliders().iter().map(move |&handle| {
                let pose = self.world.colliders[handle].position();
                BlockInstance {
                    position: pose.translation.to_array(),
                    angle: pose.rotation.angle(),
                    color: piece.color,
                }
            })
        })
    }

    pub fn execute(&mut self, action: Action) {
        match action {
            Action::RotateCw => self.rotate = Some(Rotate::Clockwise),
            Action::RotateCcw => self.rotate = Some(Rotate::Counterclockwise),
            Action::RotateStop => self.rotate = None,
            Action::MoveLeft => self.shift = Some(Shift::Left),
            Action::MoveRight => self.shift = Some(Shift::Right),
            Action::MoveStop => self.shift = None,
            Action::TrySpawn => {
                self.try_spawn();
            }
            Action::Reset => self.reset(),
        }
    }

    /// Spawn a new random piece unless one was spawned too recently.
    pub fn try_spawn(&mut self) -> bool {
        if let Some(last) = self.last_spawn
            && self.time - last < SPAWN_DELAY
        {
            return false;
        }
        let shape = &POLYOMINOS[rand::random_range(0..POLYOMINOS.len())];
        let angle = rand::random_range(0.0..std::f32::consts::TAU);
        self.spawn(shape, angle, random_color());
        true
    }

    /// Spawn a piece at the spawn point with the given orientation and hand
    /// control of it to the player.
    pub fn spawn(&mut self, shape: &[[usize; 2]], angle: f32, color: [f32; 3]) {
        let offsets: Vec<Vector> = shape
            .iter()
            .map(|&[x, y]| Vector::new(x as f32, y as f32) * BLOCK_DIST)
            .collect();
        // All blocks have the same mass, so the centre of mass is the mean.
        let local_com = offsets.iter().sum::<Vector>() / offsets.len() as f32;
        let rotation = Rotation::from_angle(angle);
        let translation = SPAWN_POINT - rotation.transform_vector(local_com);
        let handle = self.add_piece(
            Pose::from_parts(translation, rotation),
            offsets.into_iter().map(Pose::from_translation),
            color,
        );
        self.controlled = Some(handle);
        self.last_spawn = Some(self.time);
    }

    fn add_piece(
        &mut self,
        pose: Pose,
        blocks: impl IntoIterator<Item = Pose>,
        color: [f32; 3],
    ) -> RigidBodyHandle {
        let body = RigidBodyBuilder::dynamic()
            .pose(pose)
            .can_sleep(false)
            .build();
        let handle = self.world.insert_body(body);
        for local in blocks {
            self.world
                .insert_collider(block_collider().position(local), Some(handle));
        }
        self.pieces.push(Piece {
            body: handle,
            color,
        });
        handle
    }

    /// Advance the simulation by `elapsed` real time, in fixed steps.
    pub fn advance(&mut self, elapsed: Duration) {
        self.accumulator += elapsed.as_secs_f32().min(MAX_FRAME_TIME);
        while self.accumulator >= TIME_STEP {
            self.step();
            self.accumulator -= TIME_STEP;
        }
    }

    /// Run exactly one fixed simulation step.
    pub fn step(&mut self) {
        self.time += TIME_STEP;
        self.apply_controls();
        self.clear_lines();
        self.world.step();
    }

    fn apply_controls(&mut self) {
        let Some(handle) = self.controlled else {
            // Nothing to steer: forget held keys so a new piece starts still.
            self.rotate = None;
            self.shift = None;
            return;
        };
        let body = &mut self.world.bodies[handle];
        match self.rotate {
            Some(Rotate::Clockwise) => body.apply_torque_impulse(POLYOMINO_ANG_FORCE, true),
            Some(Rotate::Counterclockwise) => body.apply_torque_impulse(-POLYOMINO_ANG_FORCE, true),
            None => {}
        }
        match self.shift {
            Some(Shift::Left) => body.apply_impulse(Vector::new(-POLYOMINO_FORCE, 0.0), true),
            Some(Shift::Right) => body.apply_impulse(Vector::new(POLYOMINO_FORCE, 0.0), true),
            None => {}
        }
    }

    fn clear_lines(&mut self) {
        let mut heights: Vec<f32> = self.blocks().map(|b| b.position[1]).collect();
        heights.sort_by(f32::total_cmp);
        let lines = find_lines(&heights);
        if lines.is_empty() {
            return;
        }
        self.score += 10 * lines.len();
        self.controlled = None;
        for line in lines {
            self.split_pieces_on(line);
        }
    }

    /// Remove every block lying on `line`; pieces that lose blocks are
    /// replaced by one new body per connected group of surviving blocks.
    fn split_pieces_on(&mut self, line: Line) {
        let mut fragments = Vec::new();
        let mut i = 0;
        while i < self.pieces.len() {
            let piece = &self.pieces[i];
            let body = &self.world.bodies[piece.body];
            let all = body.colliders();
            let survivors: Vec<Pose> = all
                .iter()
                .map(|&c| &self.world.colliders[c])
                .filter(|c| (c.translation().y - line.center).abs() > line.half_height * 1.01)
                .map(|c| {
                    *c.position_wrt_parent()
                        .expect("block colliders have a parent")
                })
                .collect();
            if survivors.len() == all.len() {
                i += 1;
                continue;
            }
            let piece = self.pieces.swap_remove(i);
            let (pose, linvel, angvel, com) = {
                let b = &self.world.bodies[piece.body];
                (*b.position(), b.linvel(), b.angvel(), b.center_of_mass())
            };
            self.world.remove_body(piece.body);
            for group in connected_groups(survivors) {
                let handle = self.add_piece(pose, group, piece.color);
                // Pieces are pushed to the end; keep `i` pointing at the
                // next unvisited original piece.
                fragments.push(self.pieces.pop().expect("just pushed"));
                // A fragment keeps the parent's angular velocity. Its centre
                // of mass moved, so its linear velocity gains the parent's
                // rotation about the old centre of mass at the new one.
                let body = &mut self.world.bodies[handle];
                let delta = body.center_of_mass() - com;
                body.set_linvel(linvel + angvel * delta.perp(), true);
                body.set_angvel(angvel, true);
            }
        }
        self.pieces.extend(fragments);
    }

    pub fn reset(&mut self) {
        for piece in self.pieces.drain(..) {
            self.world.remove_body(piece.body);
        }
        self.controlled = None;
        self.rotate = None;
        self.shift = None;
        self.score = 0;
        self.last_spawn = None;
    }
}

/// Given block heights sorted ascending, find every group of
/// [`BLOCKS_PER_LINE`] blocks that lie within [`LINE_THRESHOLD`] of each other.
fn find_lines(sorted_heights: &[f32]) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut min_y = f32::NEG_INFINITY;
    for window in sorted_heights.windows(BLOCKS_PER_LINE) {
        let (low, high) = (window[0], window[BLOCKS_PER_LINE - 1]);
        // `low <= min_y` means this window overlaps a line already found,
        // which can only happen with more than BLOCKS_PER_LINE blocks at
        // the same height.
        if high - low < LINE_THRESHOLD && low > min_y {
            lines.push(Line {
                center: (low + high) / 2.0,
                half_height: (high - low) / 2.0,
            });
            min_y = high;
        }
    }
    lines
}

/// Partition block poses (relative to their body) into groups of
/// edge-adjacent blocks.
fn connected_groups(mut blocks: Vec<Pose>) -> Vec<Vec<Pose>> {
    let adjacent = |a: &Pose, b: &Pose| a.translation.distance(b.translation) < BLOCK_DIST * 1.3;
    let mut groups = Vec::new();
    while let Some(seed) = blocks.pop() {
        let mut group = vec![seed];
        let mut cursor = 0;
        while cursor < group.len() {
            let (linked, rest): (Vec<_>, Vec<_>) =
                blocks.drain(..).partition(|b| adjacent(&group[cursor], b));
            group.extend(linked);
            blocks = rest;
            cursor += 1;
        }
        groups.push(group);
    }
    groups
}

fn block_collider() -> ColliderBuilder {
    let half = BLOCK_SIZE / 2.0;
    ColliderBuilder::round_cuboid(half - CORNER_RADIUS, half - CORNER_RADIUS, CORNER_RADIUS)
        .density(POLYOMINO_DENSITY)
        .restitution(POLYOMINO_RESTITUTION)
        .friction(POLYOMINO_FRICTION)
}

/// A world with gravity, a floor and two walls.
fn create_world() -> PhysicsWorld {
    let mut world = PhysicsWorld::new();
    world.gravity = GRAVITY;
    world.integration_parameters.dt = TIME_STEP;
    let walls = [
        (Vector::new(0.0, 1.0), Vector::new(0.0, BOTTOM)),
        (Vector::new(1.0, 0.0), Vector::new(LEFT, 0.0)),
        (Vector::new(-1.0, 0.0), Vector::new(RIGHT, 0.0)),
    ];
    for (normal, at) in walls {
        let wall = ColliderBuilder::new(SharedShape::halfspace(normal))
            .translation(at)
            .restitution(WALL_RESTITUTION)
            .friction(WALL_FRICTION);
        world.insert_collider(wall, None);
    }
    world
}

/// A saturated colour with a random hue, so pieces are always easy to tell
/// apart from the black background.
fn random_color() -> [f32; 3] {
    let h = rand::random_range(0.0..6.0f32);
    let (s, v) = (0.75, 0.95);
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m]
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: [f32; 3] = [1.0; 3];
    /// Height of a block resting on the floor.
    const RESTING: f32 = BLOCK_SIZE / 2.0;

    /// Put a single block on the floor at column `col`.
    fn place_block(game: &mut Game, col: usize) {
        let x = LEFT + BLOCK_DIST * (col as f32 + 0.5);
        game.add_piece(
            Pose::from_translation(Vector::new(x, RESTING)),
            [Pose::IDENTITY],
            WHITE,
        );
    }

    #[test]
    fn spawned_piece_is_centred_on_the_spawn_point() {
        let mut game = Game::new();
        game.spawn(&POLYOMINOS[0], 0.7, WHITE);
        let body = &game.world.bodies[game.controlled.unwrap()];
        assert!(body.center_of_mass().distance(SPAWN_POINT) < 1e-4);
        assert_eq!(game.blocks().count(), POLYOMINOS[0].len());
        assert!(game.blocks().all(|b| (b.angle - 0.7).abs() < 1e-5));
    }

    #[test]
    fn spawning_is_rate_limited_by_simulated_time() {
        let mut game = Game::new();
        assert!(game.try_spawn());
        assert!(!game.try_spawn());
        for _ in 0..10 {
            game.advance(Duration::from_secs_f32((SPAWN_DELAY + 0.1) / 10.0));
        }
        assert!(game.try_spawn());
    }

    #[test]
    fn advance_runs_fixed_steps() {
        let mut game = Game::new();
        game.advance(Duration::from_millis(100));
        assert_eq!((game.time() / TIME_STEP).round() as u32, 6);
        // A long stall is clamped instead of simulating minutes at once.
        game.advance(Duration::from_secs(60));
        assert!(game.time() < 1.0);
    }

    #[test]
    fn pieces_stay_inside_the_playfield() {
        let mut game = Game::new();
        for (i, shape) in POLYOMINOS.iter().enumerate() {
            game.spawn(shape, i as f32, WHITE);
            game.execute(if i % 2 == 0 {
                Action::MoveLeft
            } else {
                Action::MoveRight
            });
            game.execute(Action::RotateCw);
            for _ in 0..90 {
                game.step();
            }
        }
        for _ in 0..600 {
            game.step();
        }
        for block in game.blocks() {
            let [x, y] = block.position;
            assert!(x > LEFT && x < RIGHT, "block escaped horizontally: {x}");
            assert!(y > BOTTOM, "block fell through the floor: {y}");
            assert!(y < TOP - 4.0, "block did not settle: {y}");
        }
    }

    #[test]
    fn find_lines_detects_full_rows_only() {
        let mut heights: Vec<f32> = (0..BLOCKS_PER_LINE)
            .map(|i| 0.5 + 0.001 * i as f32)
            .collect();
        heights.extend((0..BLOCKS_PER_LINE - 1).map(|_| 1.5));
        heights.extend((0..BLOCKS_PER_LINE).map(|i| 2.5 + 0.002 * i as f32));
        heights.sort_by(f32::total_cmp);
        let lines = find_lines(&heights);
        assert_eq!(lines.len(), 2);
        assert!((lines[0].center - 0.5055).abs() < 1e-4);
        assert!((lines[1].center - 2.511).abs() < 1e-4);
    }

    #[test]
    fn connected_groups_splits_disconnected_blocks() {
        let at = |x: f32, y: f32| Pose::from_translation(Vector::new(x, y) * BLOCK_DIST);
        let groups = connected_groups(vec![at(0.0, 0.0), at(2.0, 0.0), at(2.0, 1.0), at(2.0, 2.0)]);
        let mut sizes: Vec<usize> = groups.iter().map(Vec::len).collect();
        sizes.sort();
        assert_eq!(sizes, [1, 3]);
    }

    #[test]
    fn a_full_row_is_cleared_and_scored() {
        let mut game = Game::new();
        for col in 0..BLOCKS_PER_LINE {
            place_block(&mut game, col);
        }
        game.step();
        assert_eq!(game.score(), 10);
        assert_eq!(game.blocks().count(), 0);
        assert!(game.pieces.is_empty());
        assert_eq!(game.world.bodies.len(), 0);
    }

    #[test]
    fn a_partial_row_is_kept() {
        let mut game = Game::new();
        for col in 0..BLOCKS_PER_LINE - 1 {
            place_block(&mut game, col);
        }
        game.step();
        assert_eq!(game.score(), 0);
        assert_eq!(game.blocks().count(), BLOCKS_PER_LINE - 1);
    }

    #[test]
    fn clearing_a_row_splits_pieces_and_keeps_their_motion() {
        let mut game = Game::new();
        for col in 0..BLOCKS_PER_LINE - 1 {
            place_block(&mut game, col);
        }
        // A vertical bar whose bottom block completes the row.
        let x = LEFT + BLOCK_DIST * (BLOCKS_PER_LINE as f32 - 0.5);
        let bar = game.add_piece(
            Pose::from_translation(Vector::new(x, RESTING)),
            (0..4).map(|i| Pose::from_translation(Vector::new(0.0, i as f32 * BLOCK_DIST))),
            WHITE,
        );
        game.world.bodies[bar].set_angvel(2.0, true);
        game.controlled = Some(bar);

        game.step();

        assert_eq!(game.score(), 10);
        assert!(!game.has_controlled_piece());
        assert_eq!(game.pieces.len(), 1, "only the bar's remainder survives");
        let fragment = &game.world.bodies[game.pieces[0].body];
        assert_eq!(fragment.colliders().len(), 3);
        assert!((fragment.angvel() - 2.0).abs() < 1e-3);
        // Blocks in the cleared row are gone, the rest kept their columns.
        assert_eq!(game.blocks().count(), 3);
        assert!(game.blocks().all(|b| (b.position[0] - x).abs() < 0.05));
    }

    #[test]
    fn reset_empties_the_world() {
        let mut game = Game::new();
        game.spawn(&POLYOMINOS[1], 0.0, WHITE);
        game.execute(Action::MoveLeft);
        game.execute(Action::Reset);
        assert_eq!(game.blocks().count(), 0);
        assert_eq!(game.world.bodies.len(), 0);
        assert!(!game.has_controlled_piece());
        assert_eq!(game.shift, None);
        assert!(game.try_spawn(), "reset also lifts the spawn delay");
    }
}
