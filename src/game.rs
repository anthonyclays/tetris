use std::time::{Instant, Duration};

use na::{Vector1, Point2, Vector2, Isometry2};
use na::{Norm, Rotate};
use ncollide::shape::{self, ShapeHandle};
use nphysics2d::object::{RigidBody, RigidBodyHandle};
use nphysics2d::world::World;

use rand;
use rand::distributions::{IndependentSample, Range};

use consts::*;
use controls::Action;

#[derive(Clone)]
pub struct Tetromino {
    pub rbh: RigidBodyHandle<f32>,
    pub color: [f32; 3],
}

pub type Block = (Isometry2<f32>, ShapeHandle<Point2<f32>, Isometry2<f32>>);

impl Tetromino {
    pub fn blocks(&self) -> Vec<Block> {
        self.rbh.borrow()
            .shape().as_shape::<shape::Compound<_, _>>().unwrap()
            .shapes().iter().cloned()
            .collect()
    }

    fn retained_blocks(&self, y_pos: f32, mut threshold: f32) -> Vec<Block> {
        threshold *= 1.01; // meh
        let orig_iso = *self.rbh.borrow().position();
        self.blocks().into_iter()
            .filter(|&(iso, _)| {
                let y = (orig_iso.translation + orig_iso.rotation.rotate(&iso.translation)).y;
                (y - y_pos).abs() > threshold
            }).collect()
    }

    pub fn requires_split(&self, y_pos: f32, threshold: f32) -> bool {
        self.retained_blocks(y_pos, threshold).len() != self.blocks().len()
    }

    pub fn split_blocks(self, world: &mut World<f32>, y_pos: f32, threshold: f32) -> Vec<Self> {
        // Save the physical characteristics of the original object
        let (orig_iso, orig_lin_vel, orig_ang_vel, orig_com) = {
            let rb = self.rbh.borrow();
            (*rb.position(), rb.lin_vel(), rb.ang_vel(), *rb.center_of_mass())
        };

        self.retained_blocks(y_pos, threshold).into_iter()
            // Transform the list of remaining blocks into a forest of disjoint groups of blocks
            .fold(vec![], |mut block_forest: Vec<Vec<Block>>, (iso, shape): Block| -> Vec<Vec<Block>>{
                let (adjacent, mut others): (Vec<_>, Vec<_>) = block_forest.drain(..)
                    .partition(|block_group: &Vec<Block>|
                        block_group.iter().any(|&(other_iso, _)| {
                            (other_iso.translation - iso.translation).norm() < BLOCK_SIZE * 1.3
                        })
                    );
                let mut block_group: Vec<Block> = adjacent.into_iter().flat_map(|v| v.into_iter()).collect();
                block_group.push((iso, shape));

                others.push(block_group);

                others
            }).into_iter()
            // For each group of blocks, construct a new tetromino
            .map(|block_group: Vec<Block>| {
                let total_shape = shape::Compound::new(block_group);
                let mut rb = RigidBody::new_dynamic(total_shape, POLYOMINO_DENSITY, POLYOMINO_RESTITUTION, POLYOMINO_FRICTION);
                // Compute the position, rotation, linear velocity and angular velocity
                // of the newly generated object
                // The transform of the new object is equal to the transform of its generator.
                rb.set_transformation(orig_iso);
                // The angular velocity of the new object is simply equal to the angular
                // velocity of its generator.
                rb.set_ang_vel(orig_ang_vel);
                // The linear velocity of the new object is equal to the sum of the
                // linear velocity of its generator, and the effect of the angular velocity of its
                // generator combined with its own relative location
                let delta = *rb.center_of_mass() - orig_com;
                let cross_product = Vector2 {
                    x: -orig_ang_vel.x * delta.y,
                    y:  orig_ang_vel.x * delta.x,
                };
                rb.set_lin_vel(orig_lin_vel + cross_product);

                // Again, never deactivate a tetromino
                rb.set_deactivation_threshold(None);
                // Set the margin of the object
                rb.set_margin(0.012);

                Tetromino {
                    rbh: world.add_rigid_body(rb),
                    color: self.color,
                }
            }).collect()
    }
}

#[derive(Copy, Clone)]
pub enum RotateMove { Clockwise, Counterclockwise }
#[derive(Copy, Clone)]
pub enum Move { Left, Right, }

pub struct Game {
    world: World<f32>,
    objects: Vec<Tetromino>,
    control_object: Option<Tetromino>,
    rotate: Option<RotateMove>,
    mov: Option<Move>,
    score: usize,
    last_spawn: Option<Instant>,
    last_score: Option<Instant>,
}

impl Game {
    pub fn new() -> Self {
        Game {
            world: create_world(),
            objects: vec![],
            control_object: None,
            rotate: None,
            mov: None,
            score: 0,
            last_spawn: None,
            last_score: None,
        }
    }

    pub fn add_tetromino(&mut self) {
        self.last_spawn = Some(Instant::now());
        let mut rng = rand::thread_rng();

        // Reference to the shape of a single block
        let block_shape = ShapeHandle::new(shape::ConvexHull::new(block(BLOCK_SIZE/2.0, CORNER_RADIUS, 3)));
        // let block_shape = ShapeHandle::new(shape::Cuboid::new(Vector2::new(BLOCK_SIZE/2.0, BLOCK_SIZE/2.0)));
        // Description of all the blocks in a tetromino
        let cuboids: Vec<_> = POLYOMINOS[Range::new(0, POLYOMINOS.len()).ind_sample(&mut rng)].iter()
            // Each polyomino is described as a list of pairs of integers, describing the blocks in
            // the plane that are part of the polyomino.
            .map(|&[x, y]| {
                let translation = BLOCK_DIST * Vector2::new(x as f32, y as f32);
                let transform = Isometry2::new(translation, Vector1::new(0.0));
                (transform, block_shape.clone())
            }).collect();
        // The tetromino shape
        let total_shape = shape::Compound::new(cuboids);

        let mut rb = RigidBody::new_dynamic(total_shape, POLYOMINO_DENSITY, POLYOMINO_RESTITUTION, POLYOMINO_FRICTION);
        // Initial rotation of this tetromino
        let random_rotation = Range::new(0.0, 2.0 * ::std::f32::consts::PI).ind_sample(&mut rng);
        rb.append_rotation(&Vector1::new(random_rotation));
        // Normalize position by moving the center of mass to the origin
        let com = -rb.center_of_mass().to_vector();
        rb.append_translation(&com);
        // Move to top center
        rb.append_translation(&Vector2::new((LEFT+RIGHT)/2.0, TOP - 3.0*BLOCK_DIST));
        // Never deactivate a tetromino
        rb.set_deactivation_threshold(None);
        // Set a small margin
        rb.set_margin(0.012);

        // Register the object
        self.objects.push(Tetromino {
            rbh: self.world.add_rigid_body(rb),
            color: rand::Rand::rand(&mut rng),
        });
        self.control_object = self.objects.last().cloned();
    }

    pub fn tetrominos<'a>(&'a self) -> ::std::slice::Iter<'a, Tetromino> {
        self.objects.iter()
    }

    pub fn execute_action(&mut self, action: Action) {
        match action {
            Action::RotateCW   => self.rotate = Some(RotateMove::Clockwise),
            Action::RotateCCW  => self.rotate = Some(RotateMove::Counterclockwise),
            Action::RotateStop => self.rotate = None,

            Action::MoveLeft  => self.mov = Some(Move::Left),
            Action::MoveRight => self.mov = Some(Move::Right),
            Action::MoveStop  => self.mov = None,

            Action::TrySpawn  => { self.try_spawn(); },
            Action::GameReset => self.reset(),
        }
    }
    pub fn try_spawn(&mut self) -> bool {
        match self.last_spawn {
            Some(instant) if instant.elapsed() < Duration::from_millis(SPAWN_DELAY_MS) => false,
            _ => {
                self.add_tetromino();
                true
            }
        }
    }
    pub fn score(&self) -> usize { self.score }

    pub fn update(&mut self) {
        // If there's an object controlled by the player, move it
        if let Some(ref obj) = self.control_object {
            let ref rbh = obj.rbh;
            match self.rotate {
                Some(RotateMove::Clockwise) => rbh.borrow_mut().apply_angular_momentum(Vector1::new(POLYOMINO_ANG_FORCE)),
                Some(RotateMove::Counterclockwise) => rbh.borrow_mut().apply_angular_momentum(Vector1::new(-POLYOMINO_ANG_FORCE)),
                None => {}
            }
            match self.mov {
                Some(Move::Left)  => rbh.borrow_mut().apply_central_impulse(Vector2::new(-POLYOMINO_FORCE, 0.0)),
                Some(Move::Right) => rbh.borrow_mut().apply_central_impulse(Vector2::new(POLYOMINO_FORCE, 0.0)),
                None => {},
            }
        }
        else {
            // No object is controlled by the player, unconditionally reset movement
            self.rotate = None;
            self.mov = None;
        }

        // Removing completed lines

        // Collect the y-coordinate of all individual blocks, and sort them.
        let mut block_heights: Vec<f32> = self.objects.iter()
            .flat_map(|tetr| {
                let orig_iso = *tetr.rbh.borrow().position();
                tetr.rbh.borrow().shape()
                    .as_shape::<shape::Compound<_, _>>().unwrap()
                    .shapes().iter()
                    .map(|&(iso, _)| (orig_iso.translation + orig_iso.rotation.rotate(&iso.translation)).y)
                    .collect::<Vec<_>>().into_iter()
            }).collect();
        block_heights.sort_by(|&y1, &y2| y1.partial_cmp(&y2).unwrap());
        // All groups of BLOCKS_PER_LINE blocks that are at approximately the same height
        // form a line. Each element of `line_heights` represent a horizontal line
        // that will be deleted.
        let line_heights: Vec<(_, _)> = block_heights.iter().cloned()
            .zip(block_heights.iter().cloned().skip(BLOCKS_PER_LINE - 1))
            .filter(|&(y1, y2)| y2 - y1 < LINE_THRESHOLD)
            .fold((vec![], 0.0), |(mut lines, min_y), (y1, y2)| {
                if y1 <= min_y {
                    // This completed line overlaps with another completed line
                    // This means there's more than BLOCKS_PER_LINE blocks located
                    // on the same horizontal line - this should not happen for
                    // reasonable values of BLOCKS_PER_LINE!
                    return (lines, min_y);
                }
                let center = (y2 + y1) / 2.0;
                let threshold = (y2 - y1) / 2.0;
                lines.push((center, threshold));
                (lines, y2)
            }).0;

        if !line_heights.is_empty() {
            // At least one line was found.
            self.score += 10 * line_heights.len();
            self.control_object = None;

            let &mut Game { ref mut objects, ref mut world, .. } = self;

            for (y_pos, threshold) in line_heights {
                let mut new_objects = vec![];
                objects.retain(|tetr| {
                    if tetr.requires_split(y_pos, threshold) {
                        // This object is affected: remove it from the physics world...
                        world.remove_rigid_body(&tetr.rbh);
                        // and compute the resulting new objects + add them back in.
                        new_objects.extend_from_slice(&tetr.clone().split_blocks(world, y_pos, threshold));
                        false
                    }
                    // This object is not affected.
                    else { true }
                });
                // Add all newly generated objects back into the list of objects.
                objects.extend_from_slice(&new_objects)
            }
        }

        // Update the physics world
        self.world.step(0.016);
    }

    pub fn reset(&mut self) {
        for obj in self.objects.drain(..) {
            self.world.remove_rigid_body(&obj.rbh);
        }
        self.control_object = None;
        self.rotate = None;
        self.mov = None;
        self.score = 0;
        self.last_spawn = None;
        self.last_score = None;
    }
}

// Create and setup a new world with boundaries
fn create_world() -> World<f32> {
    let mut world = World::new();
    world.set_gravity(GRAVITY);
    let mut plane_geom = RigidBody::new_static(shape::Plane::new(-GRAVITY), WALL_RESTITUTION, WALL_FRICTION);
    plane_geom.append_translation(&Vector2::new(0.0, BOTTOM));
    world.add_rigid_body(plane_geom);
    let mut plane_geom = RigidBody::new_static(shape::Plane::new(Vector2::new(1.0, 0.0)), WALL_RESTITUTION, WALL_FRICTION);
    plane_geom.append_translation(&Vector2::new(LEFT, 0.0));
    world.add_rigid_body(plane_geom);
    let mut plane_geom = RigidBody::new_static(shape::Plane::new(Vector2::new(-1.0, 0.0)), WALL_RESTITUTION, WALL_FRICTION);
    plane_geom.append_translation(&Vector2::new(RIGHT, 0.0));
    world.add_rigid_body(plane_geom);
    world
}

// Create a list of points describing the convex hull of one block.
fn block(half_size: f32, radius: f32, points: usize) -> Vec<Point2<f32>> {
    let l1 = half_size - radius;
    let rel_pts: Vec<_> = (0..(points+1)).map(|n| {
        let angle = n as f32 * ::std::f32::consts::FRAC_PI_2 / points as f32;
        Point2::new(l1 + radius*angle.cos(), l1 + radius*angle.sin())
    }).collect();
    let mut result = rel_pts.clone();
    for &Point2 { x, y } in rel_pts.iter() {
        result.push(Point2::new(-y, x));
    }
    for &Point2 { x, y } in rel_pts.iter() {
        result.push(Point2::new(-x, -y));
    }
    for &Point2 { x, y } in rel_pts.iter() {
        result.push(Point2::new(y, -x));
    }
    result
}
