//! CPU pathfinding (docs/CPU_AI.md, step 8): the tiles a player can stand on, linked by walks, falls, drops through
//! ledges and jumps. The AI searches it each think and steers along the first link.

use crate::common::game::App;
use crate::common::global_constants::*;
use crate::common::map::{WARP_DOWN, WARP_LEFT, WARP_RIGHT, WARP_UP};
use crate::common::tile_types::*;
use crate::game::Game;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Move {
    Walk,
    Fall,
    /// Down through a ledge that is solid on top only.
    Drop,
    Jump,
    /// Into a pipe: `dir` is its `WARP_*` enter direction and `x` the pixel column of its middle.
    Warp { dir: i32, x: i32 },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Route {
    /// The player already stands on the target's tile.
    Here,
    Step { x: i32, y: i32, how: Move },
    Unreachable,
}

#[derive(Clone)]
pub struct NavGraph {
    standable: Vec<bool>,
    edges: Vec<Vec<(u16, Move)>>,
}

const W: i32 = MAPWIDTH;
const H: i32 = MAPHEIGHT;

fn idx(x: i32, y: i32) -> usize {
    (y * W + x) as usize
}

impl NavGraph {
    pub fn build(g: &Game) -> Self {
        let flags = |x: i32, y: i32| g.g_map.map(x, y);
        let solid = |x: i32, y: i32| {
            if y < 0 {
                return false;
            }
            if y >= H || !(0..W).contains(&x) {
                return true;
            }
            flags(x, y) & tile_flag_solid as i32 != 0 || !g.map_block(x as i16, y as i16).is_null()
        };
        let support = |x: i32, y: i32| {
            if y + 1 >= H {
                return false;
            }
            let below = flags(x, y + 1);
            let stand = below & (tile_flag_solid | tile_flag_solid_on_top) as i32 != 0 || !g.map_block(x as i16, (y + 1) as i16).is_null();
            stand && below & (tile_flag_death_on_top | tile_flag_super_or_player_death_top) as i32 == 0
        };
        let mut standable = vec![false; (W * H) as usize];
        for y in 0..H {
            for x in 0..W {
                standable[idx(x, y)] = !solid(x, y) && support(x, y);
            }
        }
        let stand = |x: i32, y: i32| (0..W).contains(&x) && (0..H).contains(&y) && standable[idx(x, y)];
        // The first tile a player falling down column `x` from row `y` lands on.
        let landing = |x: i32, y: i32| (y..H).take_while(|&ny| !solid(x, ny)).find(|&ny| stand(x, ny));
        let wraps = |y: i32| !solid(0, y) && !solid(W - 1, y);

        let mut edges = vec![Vec::new(); (W * H) as usize];
        for y in 0..H {
            for x in 0..W {
                if !stand(x, y) {
                    continue;
                }
                let out = &mut edges[idx(x, y)];
                for dir in [-1, 1] {
                    let mut nx = x + dir;
                    if !(0..W).contains(&nx) {
                        if !wraps(y) {
                            continue;
                        }
                        nx = nx.rem_euclid(W);
                    }
                    if stand(nx, y) {
                        out.push((idx(nx, y) as u16, Move::Walk));
                    } else if !solid(nx, y) {
                        if let Some(ny) = landing(nx, y) {
                            out.push((idx(nx, ny) as u16, Move::Fall));
                        }
                    }
                }
                if y + 1 < H && flags(x, y + 1) & tile_flag_solid_on_top as i32 != 0 && !solid(x, y + 1) {
                    if let Some(ny) = landing(x, y + 1) {
                        out.push((idx(x, ny) as u16, Move::Drop));
                    }
                }
                for dy in 0..=3 {
                    let reach: i32 = match dy {
                        3 => 2,
                        2 => 3,
                        _ => 4,
                    };
                    let top = y - dy.max(1);
                    if (top..y).any(|ry| solid(x, ry)) {
                        continue;
                    }
                    for dx in (-reach..=reach).filter(|&d| d != 0 && !(dy == 0 && d.abs() == 1)) {
                        let (tx, ty) = (x + dx, y - dy);
                        if !stand(tx, ty) {
                            continue;
                        }
                        let step = dx.signum();
                        let clear = (1..=dx.abs()).all(|i| !solid(x + step * i, top)) && (top..=ty).all(|ry| !solid(tx, ry));
                        if clear {
                            out.push((idx(tx, ty) as u16, Move::Jump));
                        }
                    }
                }
            }
        }
        let map = &g.g_map;
        for wx in 0..W {
            for wy in 0..H {
                let warp = &map.warpdata[wx as usize][wy as usize];
                let pair = [wx - 1, wx + 1].into_iter().find(|&px| {
                    (0..W).contains(&px) && map.warpdata[px as usize][wy as usize].id == warp.id && map.warpdata[px as usize][wy as usize].direction == warp.direction
                });
                let entrance = match warp.direction {
                    WARP_DOWN => Some((wx, wy - 1)),
                    WARP_UP => (wy + 1..(wy + 4).min(H)).take_while(|&ny| !solid(wx, ny)).find(|&ny| stand(wx, ny)).map(|ny| (wx, ny)),
                    WARP_LEFT => Some((wx + 1, wy)),
                    WARP_RIGHT => Some((wx - 1, wy)),
                    _ => None,
                };
                let Some((ex, ey)) = entrance.filter(|&(ex, ey)| stand(ex, ey)) else { continue };
                let middle = match (warp.direction, pair) {
                    (WARP_DOWN | WARP_UP, Some(px)) => (wx.min(px) + 1) * TILESIZE,
                    (WARP_DOWN | WARP_UP, None) => continue,
                    _ => wx * TILESIZE + TILESIZE / 2,
                };
                for exit in &map.warpexits[..map.numwarpexits.max(0) as usize] {
                    if exit.connection != warp.connection || exit.id == warp.id {
                        continue;
                    }
                    let cx = (exit.x as i32 + HALFPW).rem_euclid(App::screenWidth) / TILESIZE;
                    let cy = ((exit.y as i32 + PH - 1).max(0) / TILESIZE).min(H - 1);
                    if let Some(ly) = (cy..H).find(|&ny| stand(cx, ny)) {
                        let edge = (idx(cx, ly) as u16, Move::Warp { dir: warp.direction, x: middle });
                        if !edges[idx(ex, ey)].contains(&edge) {
                            edges[idx(ex, ey)].push(edge);
                        }
                    }
                }
            }
        }
        NavGraph { standable, edges }
    }

    /// The link out of `from` that lands farthest from `threat`, counting the screen edge as wrapping, if it gains
    /// distance over staying.
    pub fn flee(&self, from: (i32, i32), threat: (i32, i32)) -> Option<(i32, i32, Move)> {
        let far = |x: i32, y: i32| {
            let dx = (x - threat.0).abs();
            let dx = dx.min(W - dx);
            dx * dx + (y - threat.1) * (y - threat.1)
        };
        let here = far(from.0, from.1);
        self.edges[idx(from.0, from.1)]
            .iter()
            .map(|&(m, how)| (m as i32 % W, m as i32 / W, how))
            .filter(|&(x, y, _)| far(x, y) > here)
            .max_by_key(|&(x, y, _)| (far(x, y), -(y * W + x)))
    }

    /// The tile a player whose top left is (`x`, `y`) stands on, or will land on.
    pub fn cell_of(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let cx = (x + HALFPW).rem_euclid(App::screenWidth) / TILESIZE;
        let cy = ((y + PH - 1).max(0) / TILESIZE).min(H - 1);
        (cy..H).find(|&ny| self.standable[idx(cx, ny)]).map(|ny| (cx, ny))
    }

    /// The first link of the cheapest path from `from` to `to`.
    pub fn route(&self, from: (i32, i32), to: (i32, i32)) -> Route {
        let (start, goal) = (idx(from.0, from.1), idx(to.0, to.1));
        if start == goal {
            return Route::Here;
        }
        let mut cost = vec![u32::MAX; self.standable.len()];
        let mut first: Vec<Option<(u16, Move)>> = vec![None; self.standable.len()];
        let mut heap = BinaryHeap::new();
        cost[start] = 0;
        heap.push(Reverse((0u32, start as u16)));
        while let Some(Reverse((c, n))) = heap.pop() {
            let n = n as usize;
            if c > cost[n] {
                continue;
            }
            if n == goal {
                let (next, how) = first[n].unwrap();
                return Route::Step { x: next as i32 % W, y: next as i32 / W, how };
            }
            for &(m, how) in &self.edges[n] {
                let step = match how {
                    Move::Walk | Move::Fall => 1,
                    Move::Drop | Move::Jump => 2,
                    Move::Warp { .. } => 3,
                };
                let nc = c + step;
                if nc < cost[m as usize] {
                    cost[m as usize] = nc;
                    first[m as usize] = if n == start { Some((m, how)) } else { first[n] };
                    heap.push(Reverse((nc, m)));
                }
            }
        }
        Route::Unreachable
    }
}

