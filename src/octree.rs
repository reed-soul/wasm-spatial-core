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

use wasm_bindgen::prelude::*;

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

// ===========================================================================
// Build
// ===========================================================================

/// Default maximum points per leaf node.
pub const DEFAULT_MAX_POINTS_PER_NODE: u32 = 50_000;

/// Default maximum tree depth.
pub const DEFAULT_MAX_DEPTH: u32 = 21;

impl Octree {
    /// Build an octree from a flat `[x, y, z, x, y, z, ...]` position buffer.
    ///
    /// Points are reordered in-place so that each node's points occupy a
    /// contiguous range `[point_start .. point_start + point_count)`.
    ///
    /// # Arguments
    /// * `positions` — Mutable `Vec<f32>` of `[x, y, z, ...]` triples. **Will be
    ///   reordered** by this function.
    /// * `max_points_per_node` — Max points before splitting (default: 50 000).
    /// * `max_depth` — Max tree depth (default: 21).
    pub fn build(
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
        let mut reorder_map = vec![0usize; num_points];
        for (i, m) in reorder_map.iter_mut().enumerate() {
            *m = i;
        }

        let mut nodes = Vec::new();

        // Pass 1: build tree structure and reorder_map.
        Self::build_recursive(
            &mut nodes,
            positions,
            &mut reorder_map,
            bounds,
            0,
            num_points,
            max_points_per_node,
            max_depth,
            0,
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
    fn build_recursive(
        nodes: &mut Vec<OctreeNode>,
        positions: &[f32],
        reorder_map: &mut [usize],
        bounds: Bounds,
        output_start: usize,
        count: usize,
        max_points: u32,
        max_depth: u32,
        level: u32,
    ) {
        // If we should stop splitting, create a leaf.
        if count == 0 || count as u32 <= max_points || level >= max_depth {
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
                max_points,
                max_depth,
                level + 1,
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
}

/// Build an octree from a flat `[x, y, z, ...]` position buffer.
///
/// The input buffer is **not** modified (a copy is made internally).
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
) -> WasmOctree {
    let max_pts = max_points_per_node.unwrap_or(DEFAULT_MAX_POINTS_PER_NODE);
    let max_d = max_depth.unwrap_or(DEFAULT_MAX_DEPTH);
    let mut buf = positions.to_vec();
    WasmOctree {
        inner: Octree::build(&mut buf, max_pts, max_d),
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
}
