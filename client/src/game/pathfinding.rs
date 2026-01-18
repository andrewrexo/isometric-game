use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use super::chunk::ChunkManager;

/// State for tracking automated pathfinding movement
#[derive(Debug, Clone)]
pub struct PathState {
    pub path: Vec<(i32, i32)>,      // Waypoints (grid coordinates)
    pub current_index: usize,        // Current waypoint being targeted
    pub destination: (i32, i32),     // Final target
    pub pickup_target: Option<String>, // Item ID to pick up when path completes
    pub interact_target: Option<String>, // NPC ID to interact with on path completion
}

/// Node for A* priority queue
#[derive(Clone, Eq, PartialEq)]
struct Node {
    pos: (i32, i32),
    g_cost: i32,  // Cost from start
    f_cost: i32,  // g + heuristic
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap: lower f_cost = higher priority
        other.f_cost.cmp(&self.f_cost)
            .then_with(|| other.g_cost.cmp(&self.g_cost))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Manhattan distance heuristic for grid movement
fn heuristic(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

/// Find a path from start to goal using A* algorithm.
/// Returns None if no path exists or goal is too far.
///
/// # Arguments
/// * `start` - Starting grid position
/// * `goal` - Target grid position
/// * `chunk_manager` - For walkability checks
/// * `max_distance` - Maximum allowed distance (Chebyshev)
pub fn find_path(
    start: (i32, i32),
    goal: (i32, i32),
    chunk_manager: &ChunkManager,
    max_distance: i32,
) -> Option<Vec<(i32, i32)>> {
    // Early exit: already at goal
    if start == goal {
        return Some(vec![goal]);
    }

    // Early exit: goal too far (Chebyshev distance)
    let chebyshev_dist = (goal.0 - start.0).abs().max((goal.1 - start.1).abs());
    if chebyshev_dist > max_distance {
        return None;
    }

    // Early exit: goal not walkable
    if !chunk_manager.is_walkable(goal.0 as f32, goal.1 as f32) {
        return None;
    }

    // A* algorithm
    let mut open_set = BinaryHeap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();

    g_score.insert(start, 0);
    open_set.push(Node {
        pos: start,
        g_cost: 0,
        f_cost: heuristic(start, goal),
    });

    // 4-directional movement (matching server grid movement)
    let neighbors = [(0, -1), (0, 1), (-1, 0), (1, 0)];

    // Limit iterations to prevent lag
    const MAX_ITERATIONS: i32 = 1000;
    let mut iterations = 0;

    while let Some(current) = open_set.pop() {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            return None; // Too complex, give up
        }

        // Found the goal
        if current.pos == goal {
            // Reconstruct path
            let mut path = vec![goal];
            let mut current_pos = goal;
            while let Some(&prev) = came_from.get(&current_pos) {
                path.push(prev);
                current_pos = prev;
            }
            path.reverse();
            return Some(path);
        }

        let current_g = *g_score.get(&current.pos).unwrap_or(&i32::MAX);

        // Skip if we've found a better path to this node
        if current.g_cost > current_g {
            continue;
        }

        // Explore neighbors
        for (dx, dy) in neighbors {
            let neighbor = (current.pos.0 + dx, current.pos.1 + dy);

            // Check walkability
            if !chunk_manager.is_walkable(neighbor.0 as f32, neighbor.1 as f32) {
                continue;
            }

            let tentative_g = current_g + 1;

            if tentative_g < *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                // This path is better
                came_from.insert(neighbor, current.pos);
                g_score.insert(neighbor, tentative_g);

                open_set.push(Node {
                    pos: neighbor,
                    g_cost: tentative_g,
                    f_cost: tentative_g + heuristic(neighbor, goal),
                });
            }
        }
    }

    // No path found
    None
}

/// Find the best adjacent tile (N/S/E/W only) to a target position.
/// Returns the adjacent tile and the path to it, or None if no path exists.
pub fn find_path_to_adjacent(
    start: (i32, i32),
    target: (i32, i32),
    chunk_manager: &ChunkManager,
    max_distance: i32,
) -> Option<((i32, i32), Vec<(i32, i32)>)> {
    // Check all 4 cardinal directions
    let adjacent_tiles = [
        (target.0, target.1 - 1), // North
        (target.0, target.1 + 1), // South
        (target.0 - 1, target.1), // West
        (target.0 + 1, target.1), // East
    ];

    let mut best_path: Option<((i32, i32), Vec<(i32, i32)>)> = None;
    let mut best_length = i32::MAX;

    for adj in adjacent_tiles {
        // Skip if not walkable
        if !chunk_manager.is_walkable(adj.0 as f32, adj.1 as f32) {
            continue;
        }

        // Try to find a path to this adjacent tile
        if let Some(path) = find_path(start, adj, chunk_manager, max_distance) {
            let path_len = path.len() as i32;
            if path_len < best_length {
                best_length = path_len;
                best_path = Some((adj, path));
            }
        }
    }

    best_path
}
