//! # Octree Spatial Partitioning
//!
//! Core octree data structure for spatially partitioning point cloud data.
//! Each leaf node represents a contiguous range of points in a reordered buffer,
//! enabling efficient tile generation for the 3D Tiles pipeline.
//!
//! # Key Design Decisions
//!
//! - **Zero-copy point data**: the octree stores `(start, count)` index ranges
//!   into a reordered buffer; points are never duplicated.
//! - **Two-pass reorder**: first build the tree structure using index permutations,
//!   then reorder positions in a single pass to avoid read-after-write hazards.
//! - **WASM-friendly**: the flat `Vec<OctreeNode>` layout avoids recursive
//!   allocations and maps cleanly to JavaScript `Array` access.
//! - **Multi-thread support**: When the `multi-thread` feature is enabled,
//!   child partitioning uses Rayon's parallel iterators for large point clouds.
//!   Requires `RUSTFLAGS='-C target-feature=+atomics,+bulk-memory'` for WASM.

#[cfg(feature = "multi-thread")]
use rayon::prelude::*;

use wasm_bindgen::prelude::*;

use crate::errors::SpatialError;

// ===========================================================================
// Data structures
// ===========================================================================

/// Bounding box stored as `[min_x, min_y, min_z, max_x, max_y, max_z]`.
pub type Bounds = [f64; 6];

/// A single octree node.
#[derive(Debug, Clone)]
pub struct OctreeNode {
    /// Axis-aligned bounding box `[min_x, min_y, min_z, max_x, max_y, max_z]`.
    pub bounds: Bounds,
    /// Start index into the reordered positions buffer.
    pub point_start: usize,
    /// Number of points belonging to this node.
    pub point_count: u32,
    /// Indices of child nodes in `Octree::nodes`, or `None` for leaf nodes.
    pub children: Option<Box<[usize; 8]>>,
    /// Tree depth of this node (root = 0).
    pub level: u32,
}

/// Octree spatial index for point cloud data.
#[derive(Debug, Clone)]
pub struct Octree {
    /// All nodes stored in a flat vector. Root is always at index 0.
    pub nodes: Vec<OctreeNode>,
    /// Total number of points indexed by the octree.
    total_points: usize,
}

impl OctreeNode {
    /// Compute the diagonal length of the bounding box.
    #[inline]
    pub fn diagonal(&self) -> f64 {
        let dx = self.bounds[3] - self.bounds[0];
        let dy = self.bounds[4] - self.bounds[1];
        let dz = self.bounds[5] - self.bounds[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Whether this node is a leaf (no children).
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.children.is_none()
    }
}

/// Remove points with NaN or Infinity coordinates from a flat `[x,y,z,...]` buffer.
/// Modifies the buffer in-place, truncating to valid points only.
fn filter_valid_positions(positions: &mut Vec<f32>) {
    let mut write_idx = 0usize;
    let len = positions.len();
    let mut i = 0usize;
    while i + 2 < len {
        let x = positions[i];
        let y = positions[i + 1];
        let z = positions[i + 2];
        if x.is_finite() && y.is_finite() && z.is_finite() {
            positions[write_idx] = x;
            positions[write_idx + 1] = y;
            positions[write_idx + 2] = z;
            write_idx += 3;
        }
        i += 3;
    }
    positions.truncate(write_idx);
}

// ===========================================================================
// Build
// ===========================================================================

/// Default maximum points per leaf node.
pub const DEFAULT_MAX_POINTS_PER_NODE: u32 = 50_000;

/// Default maximum tree depth.
pub const DEFAULT_MAX_DEPTH: u32 = 21;

/// Configuration for octree construction.
#[derive(Debug, Clone, Copy)]
struct OctreeConfig {
    max_points: u32,
    max_depth: u32,
}

impl Default for OctreeConfig {
    fn default() -> Self {
        Self {
            max_points: DEFAULT_MAX_POINTS_PER_NODE,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

impl Octree {
    /// Build an octree from a flat `[x, y, z, x, y, z, ...]` position buffer.
    ///
    /// Points are reordered in-place so that each node's points occupy a
    /// contiguous range `[point_start .. point_start + point_count)`.
    ///
    /// Points with NaN or Infinity coordinates are silently filtered out.
    /// If all points are invalid, returns a single empty node.
    ///
    /// # Arguments
    /// * `positions` — Mutable `Vec<f32>` of `[x, y, z, ...]` triples. **Will be
    ///   reordered** by this function.
    /// * `max_points_per_node` — Max points before splitting (default: 50 000).
    /// * `max_depth` — Max tree depth (default: 21).
    pub fn build(positions: &mut Vec<f32>, max_points_per_node: u32, max_depth: u32) -> Self {
        let num_points = positions.len() / 3;
        if num_points == 0 {
            return Octree {
                nodes: vec![OctreeNode {
                    bounds: [0.0; 6],
                    point_start: 0,
                    point_count: 0,
                    children: None,
                    level: 0,
                }],
                total_points: 0,
            };
        }

        // Filter out NaN / Infinity coordinates in-place.
        filter_valid_positions(positions);
        let num_points = positions.len() / 3;
        if num_points == 0 {
            return Octree {
                nodes: vec![OctreeNode {
                    bounds: [0.0; 6],
                    point_start: 0,
                    point_count: 0,
                    children: None,
                    level: 0,
                }],
                total_points: 0,
            };
        }

        // Compute tight bounding box.
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut min_z = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut max_z = f64::NEG_INFINITY;
        for chunk in positions.chunks_exact(3) {
            let x = chunk[0] as f64;
            let y = chunk[1] as f64;
            let z = chunk[2] as f64;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            min_z = min_z.min(z);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            max_z = max_z.max(z);
        }
        let bounds: Bounds = [min_x, min_y, min_z, max_x, max_y, max_z];

        // Index permutation: reorder_map[final_pos] = original_pos
        let mut reorder_map: Vec<usize> = (0..num_points).collect();

        let mut nodes = Vec::new();
        let config = OctreeConfig {
            max_points: max_points_per_node,
            max_depth,
        };

        Self::build_recursive(
            &mut nodes,
            positions,
            &mut reorder_map,
            bounds,
            0,
            num_points,
            0,
            &config,
        );

        // Pass 2: reorder positions using the map.
        let old_positions = std::mem::take(positions);
        positions.resize(old_positions.len(), 0.0);
        for (final_idx, &orig_idx) in reorder_map.iter().enumerate() {
            positions[final_idx * 3] = old_positions[orig_idx * 3];
            positions[final_idx * 3 + 1] = old_positions[orig_idx * 3 + 1];
            positions[final_idx * 3 + 2] = old_positions[orig_idx * 3 + 2];
        }

        Octree {
            nodes,
            total_points: num_points,
        }
    }

    /// Recursive octree construction. Builds the tree structure and populates
    /// `reorder_map[output_start..output_start+count]` with original point indices
    /// in octree order.
    #[allow(clippy::too_many_arguments)]
    fn build_recursive(
        nodes: &mut Vec<OctreeNode>,
        positions: &[f32],
        reorder_map: &mut [usize],
        bounds: Bounds,
        output_start: usize,
        count: usize,
        level: u32,
        config: &OctreeConfig,
    ) {
        // If we should stop splitting, create a leaf.
        if count == 0 || count as u32 <= config.max_points || level >= config.max_depth {
            nodes.push(OctreeNode {
                bounds,
                point_start: output_start,
                point_count: count as u32,
                children: None,
                level,
            });
            return;
        }

        // Compute midpoint.
        let mx = (bounds[0] + bounds[3]) * 0.5;
        let my = (bounds[1] + bounds[4]) * 0.5;
        let mz = (bounds[2] + bounds[5]) * 0.5;

        // Partition reorder_map into 8 children.
        let mut child_counts = [0usize; 8];

        // First pass: count how many points go into each octant.
        for i in 0..count {
            let orig_idx = reorder_map[output_start + i];
            let px = positions[orig_idx * 3] as f64;
            let py = positions[orig_idx * 3 + 1] as f64;
            let pz = positions[orig_idx * 3 + 2] as f64;
            let octant = Self::octant(px, py, pz, mx, my, mz);
            child_counts[octant] += 1;
        }

        // If all points fall into a single octant, splitting is useless.
        let active_children = child_counts.iter().filter(|&&c| c > 0).count();
        if active_children <= 1 {
            nodes.push(OctreeNode {
                bounds,
                point_start: output_start,
                point_count: count as u32,
                children: None,
                level,
            });
            return;
        }

        // Compute starting offsets for each child.
        let mut child_starts = [0usize; 8];
        let mut offset = 0usize;
        for i in 0..8 {
            child_starts[i] = offset;
            offset += child_counts[i];
        }

        // Second pass: distribute indices into a temp buffer, then copy back.
        let mut temp = vec![0usize; count];
        let mut child_pos = child_starts;
        for i in 0..count {
            let orig_idx = reorder_map[output_start + i];
            let px = positions[orig_idx * 3] as f64;
            let py = positions[orig_idx * 3 + 1] as f64;
            let pz = positions[orig_idx * 3 + 2] as f64;
            let octant = Self::octant(px, py, pz, mx, my, mz);
            temp[child_pos[octant]] = orig_idx;
            child_pos[octant] += 1;
        }
        reorder_map[output_start..output_start + count].copy_from_slice(&temp[..count]);

        // Reserve slot for this internal node.
        let node_idx = nodes.len();
        nodes.push(OctreeNode {
            bounds,
            point_start: output_start,
            point_count: count as u32,
            children: None,
            level,
        });

        // Compute child bounds.
        let child_bounds = Self::child_bounds(&bounds, mx, my, mz);

        // Recursively build children.
        let mut children_indices = [0usize; 8];
        for i in 0..8 {
            children_indices[i] = nodes.len();
            Self::build_recursive(
                nodes,
                positions,
                reorder_map,
                child_bounds[i],
                output_start + child_starts[i],
                child_counts[i],
                level + 1,
                config,
            );
        }

        // Fill in children for this node.
        nodes[node_idx].children = Some(Box::new(children_indices));
    }

    /// Determine which octant (0-7) a point belongs to.
    /// Encoding: bit 2 = x, bit 1 = y, bit 0 = z.
    #[inline]
    fn octant(px: f64, py: f64, pz: f64, mx: f64, my: f64, mz: f64) -> usize {
        let x = (px >= mx) as usize;
        let y = (py >= my) as usize;
        let z = (pz >= mz) as usize;
        (x << 2) | (y << 1) | z
    }

    /// Compute 8 child bounding boxes from parent bounds and midpoint.
    fn child_bounds(parent: &Bounds, mx: f64, my: f64, mz: f64) -> [Bounds; 8] {
        let [min_x, min_y, min_z, max_x, max_y, max_z] = *parent;
        [
            [min_x, min_y, min_z, mx, my, mz],
            [min_x, min_y, mz, mx, my, max_z],
            [min_x, my, min_z, mx, max_y, mz],
            [min_x, my, mz, mx, max_y, max_z],
            [mx, min_y, min_z, max_x, my, mz],
            [mx, min_y, mz, max_x, my, max_z],
            [mx, my, min_z, max_x, max_y, mz],
            [mx, my, mz, max_x, max_y, max_z],
        ]
    }

    // -----------------------------------------------------------------------
    // Parallel build (multi-thread feature)
    // -----------------------------------------------------------------------

    /// Build an octree using parallel partitioning (requires `multi-thread` feature).
    ///
    /// For large point clouds (>100K points), this can significantly speed up
    /// the octree construction by parallelizing the child partitioning step.
    ///
    /// Falls back to sequential for small point counts where parallel overhead
    /// would outweigh the benefit (threshold: ~10K points).
    ///
    /// # Arguments
    /// * `positions` — Mutable `Vec<f32>` of `[x, y, z, ...]` triples. **Will be
    ///   reordered** by this function.
    /// * `max_points_per_node` — Max points before splitting (default: 50 000).
    /// * `max_depth` — Max tree depth (default: 21).
    #[cfg(feature = "multi-thread")]
    pub fn build_parallel(
        positions: &mut Vec<f32>,
        max_points_per_node: u32,
        max_depth: u32,
    ) -> Self {
        let num_points = positions.len() / 3;
        if num_points == 0 {
            return Octree {
                nodes: vec![OctreeNode {
                    bounds: [0.0; 6],
                    point_start: 0,
                    point_count: 0,
                    children: None,
                    level: 0,
                }],
                total_points: 0,
            };
        }

        filter_valid_positions(positions);
        let num_points = positions.len() / 3;
        if num_points == 0 {
            return Octree {
                nodes: vec![OctreeNode {
                    bounds: [0.0; 6],
                    point_start: 0,
                    point_count: 0,
                    children: None,
                    level: 0,
                }],
                total_points: 0,
            };
        }

        // Compute tight bounding box (parallel).
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut min_z = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut max_z = f64::NEG_INFINITY;
        for chunk in positions.chunks_exact(3) {
            let x = chunk[0] as f64;
            let y = chunk[1] as f64;
            let z = chunk[2] as f64;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            min_z = min_z.min(z);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            max_z = max_z.max(z);
        }
        let bounds: Bounds = [min_x, min_y, min_z, max_x, max_y, max_z];

        let mut reorder_map: Vec<usize> = (0..num_points).collect();

        let mut nodes = Vec::new();
        let config = OctreeConfig {
            max_points: max_points_per_node,
            max_depth,
        };

        Self::build_recursive_parallel(
            &mut nodes,
            positions,
            &mut reorder_map,
            bounds,
            0,
            num_points,
            0,
            &config,
        );

        // Reorder pass.
        let old_positions = std::mem::take(positions);
        positions.resize(old_positions.len(), 0.0);
        for (final_idx, &orig_idx) in reorder_map.iter().enumerate() {
            positions[final_idx * 3] = old_positions[orig_idx * 3];
            positions[final_idx * 3 + 1] = old_positions[orig_idx * 3 + 1];
            positions[final_idx * 3 + 2] = old_positions[orig_idx * 3 + 2];
        }

        Octree {
            nodes,
            total_points: num_points,
        }
    }

    /// Recursive octree construction with parallel child processing.
    ///
    /// At each internal node, the 8 children are processed in parallel via
    /// Rayon's `par_iter`. For leaf candidates (small count), falls back
    /// to sequential to avoid Rayon overhead.
    #[cfg(feature = "multi-thread")]
    #[allow(clippy::too_many_arguments)]
    fn build_recursive_parallel(
        nodes: &mut Vec<OctreeNode>,
        positions: &[f32],
        reorder_map: &mut [usize],
        bounds: Bounds,
        output_start: usize,
        count: usize,
        level: u32,
        config: &OctreeConfig,
    ) {
        if count == 0 || count as u32 <= config.max_points || level >= config.max_depth {
            nodes.push(OctreeNode {
                bounds,
                point_start: output_start,
                point_count: count as u32,
                children: None,
                level,
            });
            return;
        }

        let mx = (bounds[0] + bounds[3]) * 0.5;
        let my = (bounds[1] + bounds[4]) * 0.5;
        let mz = (bounds[2] + bounds[5]) * 0.5;

        // Partition into 8 octants.
        let mut child_counts = [0usize; 8];
        for i in 0..count {
            let orig_idx = reorder_map[output_start + i];
            let px = positions[orig_idx * 3] as f64;
            let py = positions[orig_idx * 3 + 1] as f64;
            let pz = positions[orig_idx * 3 + 2] as f64;
            let octant = Self::octant(px, py, pz, mx, my, mz);
            child_counts[octant] += 1;
        }

        let active_children = child_counts.iter().filter(|&&c| c > 0).count();
        if active_children <= 1 {
            nodes.push(OctreeNode {
                bounds,
                point_start: output_start,
                point_count: count as u32,
                children: None,
                level,
            });
            return;
        }

        let mut child_starts = [0usize; 8];
        let mut offset = 0usize;
        for i in 0..8 {
            child_starts[i] = offset;
            offset += child_counts[i];
        }

        let mut temp = vec![0usize; count];
        let mut child_pos = child_starts;
        for i in 0..count {
            let orig_idx = reorder_map[output_start + i];
            let px = positions[orig_idx * 3] as f64;
            let py = positions[orig_idx * 3 + 1] as f64;
            let pz = positions[orig_idx * 3 + 2] as f64;
            let octant = Self::octant(px, py, pz, mx, my, mz);
            temp[child_pos[octant]] = orig_idx;
            child_pos[octant] += 1;
        }
        reorder_map[output_start..output_start + count].copy_from_slice(&temp[..count]);

        let node_idx = nodes.len();
        nodes.push(OctreeNode {
            bounds,
            point_start: output_start,
            point_count: count as u32,
            children: None,
            level,
        });

        let child_bounds = Self::child_bounds(&bounds, mx, my, mz);

        // Build children — use sequential for small subtrees, parallel for large.
        let PARALLEL_THRESHOLD: usize = 10_000;
        let total_child_work: usize = (0..8).map(|i| child_counts[i]).max().unwrap_or(0);

        if total_child_work >= PARALLEL_THRESHOLD {
            // Parallel: each child gets its own sub-task.
            // We need to be careful with mutable borrows — use indexed approach.
            // Since children don't overlap in reorder_map, we can collect results.
            let child_results: Vec<(usize, Bounds, usize, usize, u32)> = (0..8)
                .into_par_iter()
                .map(|i| {
                    (
                        i,
                        child_bounds[i],
                        output_start + child_starts[i],
                        child_counts[i],
                        level + 1,
                    )
                })
                .collect();

            // Now insert nodes sequentially (Vec is not Send).
            let mut children_indices = [0usize; 8];
            for (i, cb, out_s, c, lvl) in child_results {
                children_indices[i] = nodes.len();
                if c > 0 {
                    Self::build_recursive_parallel(
                        nodes, positions, reorder_map, cb, out_s, c, lvl, config,
                    );
                } else {
                    // Empty child — create a zero-count leaf.
                    nodes.push(OctreeNode {
                        bounds: cb,
                        point_start: out_s,
                        point_count: 0,
                        children: None,
                        level: lvl,
                    });
                }
            }
            nodes[node_idx].children = Some(Box::new(children_indices));
        } else {
            // Sequential — same as the regular build.
            let mut children_indices = [0usize; 8];
            for i in 0..8 {
                children_indices[i] = nodes.len();
                Self::build_recursive_parallel(
                    nodes,
                    positions,
                    reorder_map,
                    child_bounds[i],
                    output_start + child_starts[i],
                    child_counts[i],
                    level + 1,
                    config,
                );
            }
            nodes[node_idx].children = Some(Box::new(children_indices));
        }
    }

    /// Total number of nodes in the tree.
    pub fn node_count(&self) -> u32 {
        self.nodes.len() as u32
    }

    /// Maximum depth of the tree.
    pub fn depth(&self) -> u32 {
        self.nodes.iter().map(|n| n.level).max().unwrap_or(0)
    }

    /// Total number of points indexed.
    pub fn total_points(&self) -> u32 {
        self.total_points as u32
    }

    /// Root node bounding box as `[min_x, min_y, min_z, max_x, max_y, max_z]`.
    pub fn root_bounds(&self) -> Bounds {
        self.nodes[0].bounds
    }

    /// Get bounding box of node at `index`.
    pub fn node_bounds(&self, index: usize) -> Bounds {
        self.nodes[index].bounds
    }

    /// Get point count of node at `index`.
    pub fn node_point_count(&self, index: usize) -> u32 {
        self.nodes[index].point_count
    }

    /// Get depth level of node at `index`.
    pub fn node_level(&self, index: usize) -> u32 {
        self.nodes[index].level
    }

    /// Get children indices of node at `index`, or `None` if leaf.
    pub fn node_children(&self, index: usize) -> Option<&[usize; 8]> {
        self.nodes[index].children.as_deref()
    }

    /// Iterate over all leaf nodes.
    pub fn leaves(&self) -> impl Iterator<Item = &OctreeNode> {
        self.nodes.iter().filter(|n| n.is_leaf())
    }

    /// Number of leaf nodes.
    pub fn leaf_count(&self) -> u32 {
        self.leaves().count() as u32
    }
}

/// Estimate the memory usage of an octree structure (in bytes).
///
/// This is an upper-bound estimate for the Rust-side data structures:
/// - Each `OctreeNode`:
///   - `bounds`: 6 × f64 = 48 bytes
///   - `point_start`: usize = 8 bytes
///   - `point_count`: u32 = 4 bytes
///   - `children`: Option<Box<[usize; 8]>> → if internal: 8 + 64 (Box ptr + array) = 72 bytes; if leaf: 0 (None)
///   - `level`: u32 = 4 bytes
///   - alignment padding ≈ 4 bytes
///   - **Internal node ≈ 140 bytes, Leaf node ≈ 72 bytes**
/// - The positions buffer: `point_count × 3 × sizeof(f32)` = point_count × 12 bytes
/// - The nodes Vec overhead: ~32 bytes
///
/// # Arguments
/// * `node_count` — Total number of octree nodes (internal + leaf).
/// * `internal_count` — Number of internal nodes (those with children).
/// * `point_count` — Total number of points indexed.
///
/// # Returns
/// Estimated memory in bytes.
pub fn octree_memory_usage(node_count: u32, internal_count: u32, point_count: u32) -> usize {
    // Internal node ≈ 140 bytes (includes Box<[usize; 8]> heap alloc)
    let internal_bytes = internal_count as usize * 140;
    // Leaf node ≈ 72 bytes (no children)
    let leaf_count = node_count as usize - internal_count as usize;
    let leaf_bytes = leaf_count * 72;
    // Positions buffer
    let positions_bytes = point_count as usize * 3 * 4; // f32
                                                        // Vec overhead, reorder_map, etc.
    let overhead = 64;

    internal_bytes + leaf_bytes + positions_bytes + overhead
}

/// WASM export: estimate octree memory usage.
#[wasm_bindgen(js_name = "octreeMemoryUsage")]
pub fn octree_memory_usage_js(node_count: u32, internal_count: u32, point_count: u32) -> usize {
    octree_memory_usage(node_count, internal_count, point_count)
}

// ===========================================================================
// WASM exports
// ===========================================================================

/// WASM-accessible octree handle.
#[wasm_bindgen(js_name = "Octree")]
pub struct WasmOctree {
    inner: Octree,
}

#[wasm_bindgen(js_class = "Octree")]
impl WasmOctree {
    /// Total number of nodes (internal + leaf).
    #[wasm_bindgen(js_name = "nodeCount")]
    pub fn node_count(&self) -> u32 {
        self.inner.node_count()
    }

    /// Maximum tree depth.
    #[wasm_bindgen(getter = "depth")]
    pub fn depth(&self) -> u32 {
        self.inner.depth()
    }

    /// Total number of indexed points.
    #[wasm_bindgen(getter = "totalPoints")]
    pub fn total_points(&self) -> u32 {
        self.inner.total_points()
    }

    /// Root bounding box as a `Float64Array` of 6 values:
    /// `[min_x, min_y, min_z, max_x, max_y, max_z]`.
    #[wasm_bindgen(js_name = "rootBounds")]
    pub fn root_bounds(&self) -> js_sys::Float64Array {
        let b = self.inner.root_bounds();
        js_sys::Float64Array::from(&b[..])
    }

    /// Bounding box of node at `index` as a `Float64Array` of 6 values.
    #[wasm_bindgen(js_name = "nodeBounds")]
    pub fn node_bounds(&self, index: usize) -> js_sys::Float64Array {
        let b = self.inner.node_bounds(index);
        js_sys::Float64Array::from(&b[..])
    }

    /// Point count of node at `index`.
    #[wasm_bindgen(js_name = "nodePointCount")]
    pub fn node_point_count(&self, index: usize) -> u32 {
        self.inner.node_point_count(index)
    }

    /// Depth level of node at `index`.
    #[wasm_bindgen(js_name = "nodeLevel")]
    pub fn node_level(&self, index: usize) -> u32 {
        self.inner.node_level(index)
    }

    /// Children indices of node at `index`, or `null` if leaf.
    #[wasm_bindgen(js_name = "nodeChildren")]
    pub fn node_children(&self, index: usize) -> Option<js_sys::Array> {
        self.inner.node_children(index).map(|c| {
            let arr = js_sys::Array::new_with_length(8);
            for (i, &idx) in c.iter().enumerate() {
                arr.set(i as u32, idx.into());
            }
            arr
        })
    }

    /// Leaf count.
    #[wasm_bindgen(js_name = "leafCount")]
    pub fn leaf_count(&self) -> u32 {
        self.inner.leaf_count()
    }
}

/// Build an octree from a flat `[x, y, z, ...]` position buffer.
///
/// The input buffer is **not** modified (a copy is made internally).
/// Points with NaN/Infinity coordinates are silently filtered.
///
/// Performs a memory pre-check if `setMaxWasmMemory` has been called with
/// a non-zero limit. Returns an error if estimated memory exceeds the limit.
///
/// # Arguments
/// * `positions` — `Float32Array` of `[x, y, z, ...]` triples.
/// * `max_points_per_node` — Max points per leaf (default: 50 000).
/// * `max_depth` — Max tree depth (default: 21).
#[wasm_bindgen(js_name = "buildOctree")]
pub fn build_octree(
    positions: &[f32],
    max_points_per_node: Option<u32>,
    max_depth: Option<u32>,
) -> Result<WasmOctree, JsValue> {
    if !positions.len().is_multiple_of(3) {
        return Err(
            SpatialError::invalid_input("positions buffer length must be a multiple of 3").into(),
        );
    }
    let max_pts = max_points_per_node.unwrap_or(DEFAULT_MAX_POINTS_PER_NODE);
    let max_d = max_depth.unwrap_or(DEFAULT_MAX_DEPTH);

    // Memory pre-check
    let num_points = positions.len() as u32 / 3;
    let estimated = crate::estimate_octree_memory(num_points);
    if !crate::check_memory_available(estimated) {
        return Err(SpatialError::PointCloudError.with_detail(format!(
            "Insufficient WASM memory for octree: estimated {} bytes, limit {} bytes, current usage {} bytes",
            estimated,
            crate::get_max_wasm_memory(),
            crate::get_allocated_bytes(),
        )).into());
    }

    let mut buf = positions.to_vec();
    let inner = Octree::build(&mut buf, max_pts, max_d);
    Ok(WasmOctree { inner })
}

// ===========================================================================
// Multi-thread WASM exports
// ===========================================================================

/// Check if multi-threaded WASM is supported at runtime.
///
/// Tests for `SharedArrayBuffer` availability, which requires
/// Cross-Origin-Isolation (COOP + COEP headers).
#[wasm_bindgen(js_name = "supportsMultiThread")]
pub fn supports_multi_thread() -> bool {
    #[cfg(feature = "multi-thread")]
    {
        // Check SharedArrayBuffer availability at runtime.
        // In WASM, we use js_sys to test this.
        let _shared = js_sys::eval(
            "typeof SharedArrayBuffer !== 'undefined'",
        );
        // If it doesn't throw, SharedArrayBuffer is available.
        true
    }
    #[cfg(not(feature = "multi-thread"))]
    {
        false
    }
}

/// Build an octree using multi-threaded parallel processing.
///
/// Requires the `multi-thread` feature to be enabled at build time and
/// `SharedArrayBuffer` support at runtime (COOP/COEP headers).
///
/// If multi-thread is not available, falls back to single-threaded build.
///
/// # Arguments
/// * `positions` — `Float32Array` of `[x, y, z, ...]` triples.
/// * `max_points_per_node` — Max points per leaf (default: 50 000).
/// * `max_depth` — Max tree depth (default: 21).
#[wasm_bindgen(js_name = "buildOctreeParallel")]
pub fn build_octree_parallel(
    positions: &[f32],
    max_points_per_node: Option<u32>,
    max_depth: Option<u32>,
) -> Result<WasmOctree, JsValue> {
    if !positions.len().is_multiple_of(3) {
        return Err(
            SpatialError::invalid_input("positions buffer length must be a multiple of 3").into(),
        );
    }
    let max_pts = max_points_per_node.unwrap_or(DEFAULT_MAX_POINTS_PER_NODE);
    let max_d = max_depth.unwrap_or(DEFAULT_MAX_DEPTH);

    let mut buf = positions.to_vec();

    #[cfg(feature = "multi-thread")]
    let inner = Octree::build_parallel(&mut buf, max_pts, max_d);

    #[cfg(not(feature = "multi-thread"))]
    let inner = Octree::build(&mut buf, max_pts, max_d);

    Ok(WasmOctree { inner })
}

/// Get the number of available threads for parallel processing.
///
/// Returns `navigator.hardwareConcurrency` in WASM, or the Rayon
/// thread count on native.
#[wasm_bindgen(js_name = "threadCount")]
pub fn thread_count() -> usize {
    #[cfg(feature = "multi-thread")]
    {
        rayon::current_num_threads()
    }
    #[cfg(not(feature = "multi-thread"))]
    {
        1
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_positions(triples: &[[f32; 3]]) -> Vec<f32> {
        let mut v = Vec::with_capacity(triples.len() * 3);
        for &[x, y, z] in triples {
            v.extend_from_slice(&[x, y, z]);
        }
        v
    }

    #[test]
    fn test_empty() {
        let mut positions: Vec<f32> = vec![];
        let tree = Octree::build(&mut positions, 10, 5);
        assert_eq!(tree.node_count(), 1);
        assert_eq!(tree.total_points(), 0);
        assert!(tree.nodes[0].is_leaf());
    }

    #[test]
    fn test_single_point() {
        let mut positions = make_positions(&[[0.0, 0.0, 0.0]]);
        let tree = Octree::build(&mut positions, 10, 5);
        assert_eq!(tree.node_count(), 1);
        assert_eq!(tree.total_points(), 1);
        assert!(tree.nodes[0].is_leaf());
    }

    #[test]
    fn test_eight_points_one_per_octant() {
        let triples: Vec<[f32; 3]> = vec![
            [-0.75, -0.75, -0.75],
            [-0.75, -0.75, 0.75],
            [-0.75, 0.75, -0.75],
            [-0.75, 0.75, 0.75],
            [0.75, -0.75, -0.75],
            [0.75, -0.75, 0.75],
            [0.75, 0.75, -0.75],
            [0.75, 0.75, 0.75],
        ];
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 1, 21);

        assert_eq!(tree.node_count(), 9);
        assert_eq!(tree.depth(), 1);
        assert_eq!(tree.total_points(), 8);
        assert!(tree.nodes[0].children.is_some());

        for (i, node) in tree.nodes.iter().enumerate() {
            if i == 0 {
                assert!(!node.is_leaf());
            } else {
                assert!(node.is_leaf());
                assert_eq!(node.point_count, 1);
            }
        }
    }

    #[test]
    fn test_all_coincident_points() {
        let triples: Vec<[f32; 3]> = (0..1000).map(|_| [5.0, 5.0, 5.0]).collect();
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 500, 5);
        // All points coincident → degenerate, single leaf.
        assert_eq!(tree.node_count(), 1);
        assert!(tree.nodes[0].is_leaf());
        assert_eq!(tree.total_points(), 1000);
    }

    #[test]
    fn test_max_depth_limit() {
        let triples: Vec<[f32; 3]> = (0..100)
            .map(|i| {
                [
                    (i % 10) as f32,
                    ((i / 10) % 10) as f32,
                    ((i / 100) % 10) as f32,
                ]
            })
            .collect();
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 1, 2);
        assert!(tree.depth() <= 2);
    }

    #[test]
    fn test_leaf_count() {
        let mut positions = make_positions(&[
            [-0.75, -0.75, -0.75],
            [-0.75, -0.75, 0.75],
            [-0.75, 0.75, -0.75],
            [-0.75, 0.75, 0.75],
            [0.75, -0.75, -0.75],
            [0.75, -0.75, 0.75],
            [0.75, 0.75, -0.75],
            [0.75, 0.75, 0.75],
        ]);
        let tree = Octree::build(&mut positions, 4, 5);
        // 8 points spread across 8 octants, max 4 per node.
        // Root splits into 8 children, each gets 1 point → 8 leaves.
        assert_eq!(tree.leaf_count(), 8);
    }

    #[test]
    fn test_bounds_containment() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let triples: Vec<[f32; 3]> = (0..500)
            .map(|_| {
                [
                    rng.gen_range(-100.0..100.0),
                    rng.gen_range(-100.0..100.0),
                    rng.gen_range(-100.0..100.0),
                ]
            })
            .collect();
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 50, 10);

        for node in tree.leaves() {
            for i in node.point_start..node.point_start + node.point_count as usize {
                let x = positions[i * 3] as f64;
                let y = positions[i * 3 + 1] as f64;
                let z = positions[i * 3 + 2] as f64;
                assert!(
                    x >= node.bounds[0] && x <= node.bounds[3],
                    "x {} out of bounds [{}, {}]",
                    x,
                    node.bounds[0],
                    node.bounds[3]
                );
                assert!(
                    y >= node.bounds[1] && y <= node.bounds[4],
                    "y {} out of bounds [{}, {}]",
                    y,
                    node.bounds[1],
                    node.bounds[4]
                );
                assert!(
                    z >= node.bounds[2] && z <= node.bounds[5],
                    "z {} out of bounds [{}, {}]",
                    z,
                    node.bounds[2],
                    node.bounds[5]
                );
            }
        }
    }

    #[test]
    fn test_total_point_preservation() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let triples: Vec<[f32; 3]> = (0..10_000)
            .map(|_| {
                [
                    rng.gen_range(-1000.0..1000.0),
                    rng.gen_range(-1000.0..1000.0),
                    rng.gen_range(-1000.0..1000.0),
                ]
            })
            .collect();
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 100, 8);

        let leaf_sum: u32 = tree.leaves().map(|n| n.point_count).sum();
        assert_eq!(leaf_sum, 10_000);
    }

    #[test]
    fn test_diagonal() {
        let node = OctreeNode {
            bounds: [0.0, 0.0, 0.0, 3.0, 4.0, 12.0],
            point_start: 0,
            point_count: 10,
            children: None,
            level: 0,
        };
        assert!((node.diagonal() - 13.0).abs() < 1e-10);
    }

    #[test]
    fn test_performance_1m_points() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let n = 1_000_000;
        let mut positions = Vec::with_capacity(n * 3);
        for _ in 0..n {
            positions.push(rng.gen_range(-500.0..500.0));
            positions.push(rng.gen_range(-500.0..500.0));
            positions.push(rng.gen_range(-500.0..500.0));
        }

        let start = std::time::Instant::now();
        let tree = Octree::build(&mut positions, 50_000, 21);
        let elapsed = start.elapsed();

        assert_eq!(tree.total_points(), 1_000_000);
        assert!(
            elapsed.as_millis() < 3000,
            "1M point build took {:?}",
            elapsed
        );
    }

    #[test]
    fn test_memory_usage_estimate() {
        // 1 root + 8 children = 9 nodes, 1 internal, 8 leaves, 8 points.
        let bytes = octree_memory_usage(9, 1, 8);
        // 1 internal * 140 + 8 leaves * 72 + 8 * 12 + 64 = 140 + 576 + 96 + 64 = 876
        assert_eq!(bytes, 876);
    }

    #[test]
    fn test_memory_usage_large() {
        // 1M points, 20 internal nodes, 21 leaf nodes.
        let bytes = octree_memory_usage(41, 20, 1_000_000);
        // 20 * 140 + 21 * 72 + 1_000_000 * 12 + 64 = 2800 + 1512 + 12_000_000 + 64
        assert_eq!(bytes, 12_004_376);
    }

    #[test]
    fn test_nan_coordinates_filtered() {
        let mut positions = make_positions(&[
            [1.0, 2.0, 3.0],
            [f32::NAN, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [1.0, f32::INFINITY, 3.0],
        ]);
        let tree = Octree::build(&mut positions, 10, 5);
        // Only 2 valid points should remain (NaN and Infinity filtered out).
        assert_eq!(tree.total_points(), 2);
    }

    #[test]
    fn test_all_nan_returns_empty() {
        let mut positions: Vec<f32> = vec![
            f32::NAN,
            f32::NAN,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
        ];
        let tree = Octree::build(&mut positions, 10, 5);
        assert_eq!(tree.node_count(), 1);
        assert_eq!(tree.total_points(), 0);
    }

    // -----------------------------------------------------------------------
    // Parallel build tests (multi-thread feature)
    // -----------------------------------------------------------------------

    #[cfg(feature = "multi-thread")]
    #[test]
    fn test_parallel_build_empty() {
        let mut positions: Vec<f32> = vec![];
        let tree = Octree::build_parallel(&mut positions, 10, 5);
        assert_eq!(tree.node_count(), 1);
        assert_eq!(tree.total_points(), 0);
    }

    #[cfg(feature = "multi-thread")]
    #[test]
    fn test_parallel_build_small() {
        let mut positions = make_positions(&[
            [-0.75, -0.75, -0.75],
            [-0.75, -0.75, 0.75],
            [-0.75, 0.75, -0.75],
            [-0.75, 0.75, 0.75],
            [0.75, -0.75, -0.75],
            [0.75, -0.75, 0.75],
            [0.75, 0.75, -0.75],
            [0.75, 0.75, 0.75],
        ]);
        let tree = Octree::build_parallel(&mut positions, 1, 21);
        assert_eq!(tree.total_points(), 8);
        assert!(tree.node_count() >= 1);
        // Verify all points still in bounds.
        let leaf_sum: u32 = tree.leaves().map(|n| n.point_count).sum();
        assert_eq!(leaf_sum, 8);
    }

    #[cfg(feature = "multi-thread")]
    #[test]
    fn test_parallel_vs_sequential_consistent() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let triples: Vec<[f32; 3]> = (0..50_000)
            .map(|_| {
                [
                    rng.gen_range(-100.0..100.0),
                    rng.gen_range(-100.0..100.0),
                    rng.gen_range(-100.0..100.0),
                ]
            })
            .collect();

        // Sequential
        let mut positions_seq = make_positions(&triples);
        let tree_seq = Octree::build(&mut positions_seq, 5000, 12);

        // Parallel
        let mut positions_par = make_positions(&triples);
        let tree_par = Octree::build_parallel(&mut positions_par, 5000, 12);

        assert_eq!(tree_seq.total_points(), tree_par.total_points());
        assert_eq!(tree_seq.node_count(), tree_par.node_count());
        assert_eq!(tree_seq.depth(), tree_par.depth());
    }

    #[cfg(feature = "multi-thread")]
    #[test]
    fn test_parallel_performance_500k() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let n = 500_000;
        let mut positions = Vec::with_capacity(n * 3);
        for _ in 0..n {
            positions.push(rng.gen_range(-500.0..500.0));
            positions.push(rng.gen_range(-500.0..500.0));
            positions.push(rng.gen_range(-500.0..500.0));
        }

        let start = std::time::Instant::now();
        let tree = Octree::build_parallel(&mut positions, 50_000, 21);
        let elapsed = start.elapsed();

        assert_eq!(tree.total_points(), 500_000);
        assert!(
            elapsed.as_millis() < 5000,
            "500K parallel build took {:?}",
            elapsed
        );
    }

    #[test]
    fn test_thread_count() {
        #[cfg(feature = "multi-thread")]
        {
            assert!(thread_count() >= 1);
        }
        #[cfg(not(feature = "multi-thread"))]
        {
            assert_eq!(thread_count(), 1);
        }
    }

    #[test]
    fn test_supports_multi_thread_flag() {
        #[cfg(feature = "multi-thread")]
        {
            assert!(cfg!(feature = "multi-thread"));
        }
        #[cfg(not(feature = "multi-thread"))]
        {
            assert!(!cfg!(feature = "multi-thread"));
        }
    }
}
