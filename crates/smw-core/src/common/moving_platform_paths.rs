//! Paths that move platforms: back-and-forth lines, endless lines that wrap the screen, ellipses, and falling.

use crate::game::Game;
use crate::common::map::PlatformId;
use crate::common::eyecandy_styles::SpawnStyle;
use crate::common::game::App;
use crate::common::global_constants::*;
use crate::common::math::trig::{atan2f, cosf, sinf};
use crate::common::math::vec2::Vec2f;
use crate::common::movingplatform::MovingPlatform;
use crate::common::object_base::cap_falling_velocity;
use crate::globals::{Ptr, Aliased};
use std::any::Any;

/// Kind of platform path; the first three values are the map file's path type IDs.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum PlatformPathType {
    #[default]
    Straight,
    StraightContinuous,
    Ellipse,
    Falling,
}

/// Velocity of `speed` along `angle` (radians), with components under 0.01 snapped to 0.
fn calc_velocity(speed: f32, angle: f32) -> Vec2f {
    let mut vel = Vec2f::new(speed * cosf(angle), speed * sinf(angle));

    if vel.x.abs() < 0.01 {
        vel.x = 0.0;
    }
    if vel.y.abs() < 0.01 {
        vel.y = 0.0;
    }

    vel
}

#[derive(Clone)]

/// State shared by every path. Each array holds two tracks: 0 is the platform's position, 1 a lookahead that
/// runs ahead by the spawn animation's length so spawn checks see where the platform will be.
pub struct MovingPlatformPath {
    /// Start and end points in pixels; an ellipse keeps its center in `m_startPos`.
    pub m_startPos: Vec2f,
    pub m_endPos: Vec2f,
    /// Speed per 62.5 Hz tick as the map file stores it (pixels, or radians for an ellipse); motion scales it with `vel`.
    pub m_speed: f32,

    pub m_velocity: [Vec2f; 2],
    pub m_currentPos: [Vec2f; 2],
}

impl MovingPlatformPath {
    /// A path base; `preview` halves positions and speed for the half-size map preview.
    pub fn new(speed: f32, startPos: Vec2f, endPos: Vec2f, preview: bool) -> Self {
        let mut p = MovingPlatformPath {
            m_startPos: startPos,
            m_endPos: endPos,
            m_speed: speed,
            m_velocity: [Vec2f::default(); 2],
            m_currentPos: [Vec2f::default(); 2],
        };
        if preview {
            p.m_startPos /= 2.0;
            p.m_endPos /= 2.0;
            p.m_speed /= 2.0;
        }
        p
    }

    pub fn speed(&self) -> f32 {
        self.m_speed
    }
    pub fn current_pos0(&self) -> &Vec2f {
        &self.m_currentPos[0]
    }
    pub fn current_pos1(&self) -> &Vec2f {
        &self.m_currentPos[1]
    }
    pub fn velocity0(&self) -> &Vec2f {
        &self.m_velocity[0]
    }
}

/// Cloning for boxed paths.
pub trait ClonePath {
    fn clone_box(&self) -> Box<dyn MovingPlatformPathTrait>;
}

impl<T: MovingPlatformPathTrait + Clone> ClonePath for T {
    fn clone_box(&self) -> Box<dyn MovingPlatformPathTrait> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn MovingPlatformPathTrait> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A platform path. `move` advances track `type` (0 or 1) one tick.
pub trait MovingPlatformPathTrait: Any + ClonePath {
    fn path(&self) -> &MovingPlatformPath;
    fn path_mut(&mut self) -> &mut MovingPlatformPath;
    fn as_any(&mut self) -> &mut dyn Any;

    /// The path's kind; not named `type_id` because `Any` already has one.
    fn path_type_id(&self) -> PlatformPathType;
    fn r#move(&mut self, g: &mut Game, r#type: i16, platform: Ptr<MovingPlatform>) -> bool;
    fn reset(&mut self, g: &mut Game, platform: Ptr<MovingPlatform>) {
        moving_platform_path_reset(g, self, platform)
    }
}

impl std::ops::Deref for dyn MovingPlatformPathTrait {
    type Target = MovingPlatformPath;
    fn deref(&self) -> &MovingPlatformPath {
        self.path()
    }
}

impl std::ops::DerefMut for dyn MovingPlatformPathTrait {
    fn deref_mut(&mut self) -> &mut MovingPlatformPath {
        self.path_mut()
    }
}

/// Advances the lookahead track by the spawn animation's length: 36 ticks for door spawns, 50 for swirl.
pub fn moving_platform_path_reset<T: MovingPlatformPathTrait + ?Sized>(g: &mut Game, this: &mut T, platform: Ptr<MovingPlatform>) {
    if g.game_values.spawnstyle == SpawnStyle::Door {
        for _ in 0..36 {
            this.r#move(g, 1, platform);
        }
    } else if g.game_values.spawnstyle == SpawnStyle::Swirl {
        for _ in 0..50 {
            this.r#move(g, 1, platform);
        }
    }
}

macro_rules! path_plumbing {
    () => {
        fn path(&self) -> &MovingPlatformPath {
            &self.base
        }
        fn path_mut(&mut self) -> &mut MovingPlatformPath {
            &mut self.base
        }
        fn as_any(&mut self) -> &mut dyn Any {
            self
        }
    };
}

/// The end of a straight path a track is heading for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GoalPoint {
    Start,
    End,
}

//------------------------------------------------------------------------------
// Straight Path
//------------------------------------------------------------------------------

#[derive(Clone)]

/// Moves back and forth between the start and end points.
pub struct StraightPath {
    pub base: MovingPlatformPath,
    m_angle: f32,
    m_steps: i16,
    m_currentStep: [u16; 2],
    m_goalPoint: [GoalPoint; 2],
}

/// The angle/length setup shared by `StraightPath` and `StraightPathContinuous` constructors.
fn straight_angle_and_length(base: &MovingPlatformPath) -> (f32, f32) {
    let width = base.m_endPos.x - base.m_startPos.x;
    let height = base.m_endPos.y - base.m_startPos.y;
    let angle;
    let length;

    if width == 0.0 {
        angle = if height > 0.0 { HALF_PI } else { THREE_HALF_PI };
        length = height.abs();
    } else if height == 0.0 {
        angle = if width > 0.0 { 0.0 } else { PI };
        length = width.abs();
    } else {
        angle = atan2f(height, width);
        length = (height * height + width * width).sqrt();
    }
    (angle, length)
}

impl StraightPath {
    pub fn new(speed: f32, startPos: Vec2f, endPos: Vec2f, preview: bool) -> Self {
        let base = MovingPlatformPath::new(speed, startPos, endPos, preview);
        let (m_angle, length) = straight_angle_and_length(&base);
        let m_steps = (length / vel(base.m_speed)) as i16 + 1;

        let mut p = StraightPath { base, m_angle, m_steps, m_currentStep: [0, 0], m_goalPoint: [GoalPoint::Start, GoalPoint::Start] };

        for r#type in 0..2 {
            p.set_velocity(r#type);
        }
        p
    }

    pub fn start_pos(&self) -> &Vec2f {
        &self.base.m_startPos
    }
    pub fn end_pos(&self) -> &Vec2f {
        &self.base.m_endPos
    }

    fn goal(&self, g: GoalPoint) -> Vec2f {
        match g {
            GoalPoint::Start => self.base.m_startPos,
            GoalPoint::End => self.base.m_endPos,
        }
    }

    fn set_velocity(&mut self, r#type: usize) {
        self.base.m_velocity[r#type] = calc_velocity(vel(self.base.m_speed), self.m_angle);

        if self.m_goalPoint[r#type] == GoalPoint::Start {
            self.base.m_velocity[r#type] *= -1.0;
        }
    }
}

impl MovingPlatformPathTrait for StraightPath {
    path_plumbing!();

    fn path_type_id(&self) -> PlatformPathType {
        PlatformPathType::Straight
    }

    fn r#move(&mut self, g: &mut Game, r#type: i16, platform: Ptr<MovingPlatform>) -> bool {
        let t = r#type as usize;
        let v = self.base.m_velocity[t];
        self.base.m_currentPos[t] += v;
        self.m_currentStep[t] = self.m_currentStep[t].wrapping_add(1);

        if self.m_currentStep[t] as i32 >= self.m_steps as i32 {
            self.m_currentStep[t] = 0;
            self.base.m_currentPos[t] = self.goal(self.m_goalPoint[t]);
            self.m_goalPoint[t] = if self.m_goalPoint[t] == GoalPoint::End { GoalPoint::Start } else { GoalPoint::End };
            self.set_velocity(t);
        }

        false
    }

    fn reset(&mut self, g: &mut Game, platform: Ptr<MovingPlatform>) {
        for t in 0..2 {
            self.m_currentStep[t] = 0;
            self.base.m_currentPos[t] = self.base.m_startPos;
            self.m_goalPoint[t] = GoalPoint::End;
            self.set_velocity(t);
        }

        moving_platform_path_reset(g, self, platform);
    }
}

//------------------------------------------------------------------------------
// Straight Path Continuous
//------------------------------------------------------------------------------

#[derive(Clone)]

/// Moves from the start point along `m_angle` forever, wrapping around the screen edges.
pub struct StraightPathContinuous {
    pub base: MovingPlatformPath,
    m_angle: f32,
    m_steps: i16,
    m_edge: Vec2f,
    m_currentStep: [u16; 2],
    m_goalPoint: [GoalPoint; 2],
}

impl StraightPathContinuous {
    pub fn new(speed: f32, startPos: Vec2f, angle: f32, preview: bool) -> Self {
        let base = MovingPlatformPath::new(speed, startPos, Vec2f::zero(), preview);
        let (m_angle, length) = straight_angle_and_length(&base);
        let m_steps = (length / vel(base.m_speed)) as i16 + 1;

        let mut p = StraightPathContinuous {
            base,
            m_angle,
            m_steps,
            m_edge: Vec2f::default(),
            m_currentStep: [0, 0],
            m_goalPoint: [GoalPoint::Start, GoalPoint::Start],
        };

        for t in 0..2 {
            p.set_velocity(t);
        }

        p.m_angle = angle;

        for t in 0..2 {
            p.m_goalPoint[t] = GoalPoint::End;
            p.set_velocity(t);
        }

        p.m_edge.x = App::screenWidth as f32;
        p.m_edge.y = App::screenHeight as f32;
        if preview {
            p.m_edge /= 2.0;
        }
        p
    }

    pub fn start_pos(&self) -> &Vec2f {
        &self.base.m_startPos
    }
    pub fn end_pos(&self) -> &Vec2f {
        &self.base.m_endPos
    }
    pub fn angle(&self) -> f32 {
        self.m_angle
    }

    fn set_velocity(&mut self, t: usize) {
        self.base.m_velocity[t] = calc_velocity(vel(self.base.m_speed), self.m_angle);

        if self.m_goalPoint[t] == GoalPoint::Start {
            self.base.m_velocity[t] *= -1.0;
        }
    }
}

impl MovingPlatformPathTrait for StraightPathContinuous {
    path_plumbing!();

    fn path_type_id(&self) -> PlatformPathType {
        PlatformPathType::StraightContinuous
    }

    fn r#move(&mut self, g: &mut Game, r#type: i16, platform: Ptr<MovingPlatform>) -> bool {
        let t = r#type as usize;
        let v = self.base.m_velocity[t];
        self.base.m_currentPos[t] += v;

        let pos = &mut self.base.m_currentPos[t];

        let dx = pos.x - platform.iHalfWidth as f32;
        if dx < 0.0 {
            pos.x += self.m_edge.x;
        } else if dx >= self.m_edge.x {
            pos.x -= self.m_edge.x;
        }

        if pos.y + (platform.iHalfHeight as f32) < 0.0 {
            pos.y += self.m_edge.y + platform.iHeight as f32;
        } else if pos.y - platform.iHalfHeight as f32 >= self.m_edge.y {
            pos.y -= self.m_edge.y + platform.iHeight as f32;
        }

        false
    }

    fn reset(&mut self, g: &mut Game, platform: Ptr<MovingPlatform>) {
        for t in 0..2 {
            self.base.m_currentPos[t] = self.base.m_startPos;
        }

        moving_platform_path_reset(g, self, platform);
    }
}

//------------------------------------------------------------------------------
// Ellipse Path
//------------------------------------------------------------------------------

#[derive(Clone)]

/// Circles an ellipse around the center, starting at `m_startAngle` (radians).
pub struct EllipsePath {
    pub base: MovingPlatformPath,
    m_radius: Vec2f,
    m_startAngle: f32,
    m_angle: [f32; 2],
}

impl EllipsePath {
    pub fn new(speed: f32, angle: f32, radius: Vec2f, centerPos: Vec2f, preview: bool) -> Self {
        let base = MovingPlatformPath::new(speed, centerPos, Vec2f::zero(), preview);
        let mut p = EllipsePath { base, m_radius: radius, m_startAngle: angle, m_angle: [0.0; 2] };

        if preview {
            p.m_radius /= 2.0;
            p.base.m_speed *= 2.0;
        }

        for t in 0..2 {
            p.m_angle[t] = p.m_startAngle;
            p.set_position(t as i16);
        }
        p
    }

    pub fn center_pos(&self) -> &Vec2f {
        &self.base.m_startPos
    }
    pub fn radius(&self) -> &Vec2f {
        &self.m_radius
    }
    pub fn start_angle(&self) -> f32 {
        self.m_startAngle
    }

    pub fn set_position(&mut self, r#type: i16) {
        let t = r#type as usize;
        self.base.m_currentPos[t].x = self.m_radius.x * cosf(self.m_angle[t]) + self.base.m_startPos.x;
        self.base.m_currentPos[t].y = self.m_radius.y * sinf(self.m_angle[t]) + self.base.m_startPos.y;
    }
}

impl MovingPlatformPathTrait for EllipsePath {
    path_plumbing!();

    fn path_type_id(&self) -> PlatformPathType {
        PlatformPathType::Ellipse
    }

    fn r#move(&mut self, g: &mut Game, r#type: i16, platform: Ptr<MovingPlatform>) -> bool {
        let t = r#type as usize;
        let oldPos = self.base.m_currentPos[t];

        self.m_angle[t] += vel(self.base.m_speed);

        if self.base.m_speed < 0.0 {
            while self.m_angle[t] < 0.0 {
                self.m_angle[t] += TWO_PI;
            }
        } else {
            while self.m_angle[t] >= TWO_PI {
                self.m_angle[t] -= TWO_PI;
            }
        }

        self.set_position(r#type);

        self.base.m_velocity[t] = self.base.m_currentPos[t] - oldPos;

        false
    }

    fn reset(&mut self, g: &mut Game, platform: Ptr<MovingPlatform>) {
        for t in 0..2 {
            self.m_angle[t] = self.m_startAngle;
            self.set_position(t as i16);
        }

        moving_platform_path_reset(g, self, platform);
    }
}

//------------------------------------------------------------------------------
// Falling path (for falling donut blocks)
//------------------------------------------------------------------------------

#[derive(Clone)]

/// Falls under gravity until the platform leaves the bottom of the screen, then marks it dead.
pub struct FallingPath {
    pub base: MovingPlatformPath,
    _alias: Aliased,
}

impl FallingPath {
    pub fn new(startPos: Vec2f) -> Self {
        FallingPath { _alias: Aliased::new(), base: MovingPlatformPath::new(0.0, startPos, Vec2f::zero(), false) }
    }
}

impl MovingPlatformPathTrait for FallingPath {
    path_plumbing!();

    fn path_type_id(&self) -> PlatformPathType {
        PlatformPathType::Falling
    }

    fn r#move(&mut self, g: &mut Game, r#type: i16, platform: Ptr<MovingPlatform>) -> bool {
        let mut gp = g.ptr();
        let t = r#type as usize;
        self.base.m_velocity[t].y = cap_falling_velocity(self.base.m_velocity[t].y + GRAVITATION);

        let mut platform = platform;
        if platform.fy - platform.iHalfHeight as f32 >= App::screenHeight as f32 {
            for player in gp.players.iter_mut() {
                if g.player(*player).platform == platform.id {
                    g.player(*player).platform = PlatformId::NONE;
                    g.player(*player).vely = self.base.m_velocity[t].y;
                }
            }

            platform.fDead = true;
        }

        self.base.m_currentPos[t].y += self.base.m_velocity[t].y;

        false
    }

    fn reset(&mut self, g: &mut Game, platform: Ptr<MovingPlatform>) {
        for t in 0..2 {
            self.base.m_currentPos[t] = self.base.m_startPos;
        }
    }
}
