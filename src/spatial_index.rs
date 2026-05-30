//! Frontend Spatial Indexing
//!
//! Provides a WebAssembly-backed R-Tree for fast spatial queries
//! (bounding box searches) over large coordinate datasets.
//!
//! Allows Web3D engines to selectively render or load features
//! that are currently within the camera's viewport, avoiding the
//! overhead of processing millions of out-of-view vertices.

use js_sys::{Float64Array, Uint32Array};
use rstar::primitives::{GeomWithData, Line};
use rstar::{RTree, AABB};
use wasm_bindgen::prelude::*;

type Point2D = [f64; 2];
type IndexedPoint = GeomWithData<Point2D, u32>;
type IndexedEdge = GeomWithData<Line<Point2D>, u32>;

/// A high-performance spatial index using an R-Tree.
#[wasm_bindgen]
pub struct SpatialIndex {
    tree: RTree<IndexedPoint>,
}

#[wasm_bindgen]
impl SpatialIndex {
    /// Build a spatial index from a flat Float64Array of coordinates `[lng0, lat0, lng1, lat1, ...]`.
    /// Each coordinate pair is assigned an ID equal to its index (i.e. `0` for the first pair, `1` for the second).
    #[wasm_bindgen(constructor)]
    pub fn new(coords: &Float64Array) -> SpatialIndex {
        let len = coords.length() as usize;
        let mut buf = vec![0.0; len];
        coords.copy_to(&mut buf);

        let mut points = Vec::with_capacity(len / 2);
        for (i, chunk) in buf.chunks_exact(2).enumerate() {
            let pt = [chunk[0], chunk[1]];
            points.push(IndexedPoint::new(pt, i as u32));
        }

        let tree = RTree::bulk_load(points);
        SpatialIndex { tree }
    }

    /// Search for all points within a given bounding box.
    /// Returns a `Uint32Array` containing the IDs of the points.
    #[wasm_bindgen(js_name = "searchBBox")]
    pub fn search_bbox(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Uint32Array {
        let envelope = AABB::from_corners([min_x, min_y], [max_x, max_y]);
        let mut results = Vec::new();

        for point in self.tree.locate_in_envelope(&envelope) {
            results.push(point.data);
        }

        let result_array = Uint32Array::new_with_length(results.len() as u32);
        result_array.copy_from(&results);
        result_array
    }

    /// Get the total number of points in the index.
    #[wasm_bindgen]
    pub fn size(&self) -> u32 {
        self.tree.size() as u32
    }

    /// Find the nearest point to a given query coordinate.
    /// Returns the ID of the nearest point, or `null` if the index is empty.
    #[wasm_bindgen(js_name = "nearestNeighbor")]
    pub fn nearest_neighbor(&self, query_x: f64, query_y: f64) -> Option<u32> {
        let query_point = [query_x, query_y];
        let nearest = self.tree.nearest_neighbor(&query_point)?;
        Some(nearest.data)
    }

    /// Find the K nearest neighbors to a given query coordinate.
    /// Returns a `Uint32Array` containing the IDs, ordered by distance (nearest first).
    /// If `k` is greater than the number of points, returns all points.
    #[wasm_bindgen(js_name = "kNearestNeighbors")]
    pub fn k_nearest_neighbors(&self, query_x: f64, query_y: f64, k: u32) -> Uint32Array {
        let query_point = [query_x, query_y];
        let nearest_iter = self.tree.nearest_neighbor_iter(&query_point);
        let results: Vec<u32> = nearest_iter.take(k as usize).map(|p| p.data).collect();

        let result_array = Uint32Array::new_with_length(results.len() as u32);
        result_array.copy_from(&results);
        result_array
    }
}

// ===========================================================================
// SpatialEdgeIndex — R-Tree index for line segments (LineString edges)
// ===========================================================================

/// A spatial index for 2D line segments using an R-Tree.
///
/// Indexes individual edges (line segments) from LineString geometries.
/// Supports bounding box queries to find all edges that intersect with
/// a given rectangular area. Useful for viewport-based progressive loading
/// of road networks, pipelines, and other linear features.
#[wasm_bindgen]
pub struct SpatialEdgeIndex {
    tree: RTree<IndexedEdge>,
}

#[wasm_bindgen]
impl SpatialEdgeIndex {
    /// Build a spatial edge index from line segments.
    ///
    /// Input format: a flat `Float64Array` of line segment endpoints
    /// `[x0, y0, x1, y1, x2, y2, x3, y3, ...]` where each consecutive
    /// pair of 2D points forms an edge (line segment).
    ///
    /// Each edge is assigned an ID equal to its sequential index
    /// (0 for the first edge, 1 for the second, etc.).
    #[wasm_bindgen(constructor)]
    pub fn new(segments: &Float64Array) -> SpatialEdgeIndex {
        let len = segments.length() as usize;
        let mut buf = vec![0.0; len];
        segments.copy_to(&mut buf);

        // Each edge is 4 floats: (x0, y0, x1, y1)
        let edge_count = buf.chunks_exact(4).count();
        let mut edges = Vec::with_capacity(edge_count);

        for (i, chunk) in buf.chunks_exact(4).enumerate() {
            let from = [chunk[0], chunk[1]];
            let to = [chunk[2], chunk[3]];
            let line = Line::new(from, to);
            edges.push(IndexedEdge::new(line, i as u32));
        }

        let tree = RTree::bulk_load(edges);
        SpatialEdgeIndex { tree }
    }

    /// Search for all edges within a given bounding box.
    /// Returns a `Uint32Array` containing the IDs of matching edges.
    ///
    /// An edge matches if its bounding box intersects the query envelope.
    #[wasm_bindgen(js_name = "searchBBox")]
    pub fn search_bbox(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Uint32Array {
        let envelope = AABB::from_corners([min_x, min_y], [max_x, max_y]);
        let mut results = Vec::new();

        for edge in self.tree.locate_in_envelope(&envelope) {
            results.push(edge.data);
        }

        let result_array = Uint32Array::new_with_length(results.len() as u32);
        result_array.copy_from(&results);
        result_array
    }

    /// Get the total number of edges in the index.
    #[wasm_bindgen]
    pub fn size(&self) -> u32 {
        self.tree.size() as u32
    }

    /// Find the nearest edge to a given query coordinate.
    /// Returns the ID of the nearest edge, or `null` if the index is empty.
    ///
    /// Distance is measured as the minimum Euclidean distance from the
    /// query point to any point on the edge.
    #[wasm_bindgen(js_name = "nearestNeighbor")]
    pub fn nearest_neighbor(&self, query_x: f64, query_y: f64) -> Option<u32> {
        let query_point = [query_x, query_y];
        let nearest = self.tree.nearest_neighbor(&query_point)?;
        Some(nearest.data)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_index() {
        let points = vec![
            IndexedPoint::new([0.0, 0.0], 0),
            IndexedPoint::new([10.0, 10.0], 1),
            IndexedPoint::new([20.0, 20.0], 2),
        ];

        let tree = RTree::bulk_load(points);
        let envelope = AABB::from_corners([5.0, 5.0], [15.0, 15.0]);
        let results: Vec<_> = tree.locate_in_envelope(&envelope).collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].data, 1);
    }

    #[test]
    fn test_nearest_neighbor() {
        let points = vec![
            IndexedPoint::new([0.0, 0.0], 0),
            IndexedPoint::new([10.0, 10.0], 1),
            IndexedPoint::new([20.0, 20.0], 2),
        ];

        let tree = RTree::bulk_load(points);

        // Query near (10, 10) — should return id 1
        let nearest = tree.nearest_neighbor(&[11.0, 9.0]).unwrap();
        assert_eq!(nearest.data, 1);

        // Query near (0, 0) — should return id 0
        let nearest = tree.nearest_neighbor(&[0.5, 0.5]).unwrap();
        assert_eq!(nearest.data, 0);
    }

    #[test]
    fn test_k_nearest_neighbors() {
        let points = vec![
            IndexedPoint::new([0.0, 0.0], 0),
            IndexedPoint::new([5.0, 0.0], 1),
            IndexedPoint::new([10.0, 0.0], 2),
            IndexedPoint::new([100.0, 100.0], 3),
        ];

        let tree = RTree::bulk_load(points);
        let nearest_iter = tree.nearest_neighbor_iter(&[4.0, 1.0]);
        let results: Vec<u32> = nearest_iter.take(2).map(|p| p.data).collect();

        // Nearest to (4,1) should be id=1 (5,0), then id=0 (0,0)
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], 1);
        assert_eq!(results[1], 0);
    }

    // ── SpatialEdgeIndex tests ─────────────────────────────────────────

    #[test]
    fn test_edge_index_bbox() {
        // Edge 0: (0,0)→(10,10) — fully inside bbox [0,0,15,15]
        // Edge 1: (20,20)→(30,30) — fully outside bbox [0,0,15,15]
        // Edge 2: (5,5)→(15,15) — fully inside bbox [0,0,15,15]
        let edges = vec![
            IndexedEdge::new(Line::new([0.0, 0.0], [10.0, 10.0]), 0),
            IndexedEdge::new(Line::new([20.0, 20.0], [30.0, 30.0]), 1),
            IndexedEdge::new(Line::new([5.0, 5.0], [15.0, 15.0]), 2),
        ];
        let tree = RTree::bulk_load(edges);

        // Use a larger query box that fully contains edges 0 and 2
        let envelope = AABB::from_corners([-1.0, -1.0], [16.0, 16.0]);
        let mut ids: Vec<u32> = tree.locate_in_envelope(&envelope).map(|e| e.data).collect();
        ids.sort();

        assert!(ids.contains(&0), "Edge 0 should be in bbox");
        assert!(ids.contains(&2), "Edge 2 should be in bbox");
        assert!(!ids.contains(&1), "Edge 1 should NOT be in bbox");
    }

    #[test]
    fn test_edge_index_nearest() {
        let edges = vec![
            IndexedEdge::new(Line::new([0.0, 0.0], [10.0, 0.0]), 0),
            IndexedEdge::new(Line::new([0.0, 10.0], [10.0, 10.0]), 1),
        ];
        let tree = RTree::bulk_load(edges);

        // Query point (5, 1) — nearest should be edge 0 (y=0)
        let nearest = tree.nearest_neighbor(&[5.0, 1.0]).unwrap();
        assert_eq!(nearest.data, 0);

        // Query point (5, 9) — nearest should be edge 1 (y=10)
        let nearest = tree.nearest_neighbor(&[5.0, 9.0]).unwrap();
        assert_eq!(nearest.data, 1);
    }

    #[test]
    fn test_edge_index_empty() {
        let edges: Vec<IndexedEdge> = vec![];
        let tree = RTree::bulk_load(edges);
        assert_eq!(tree.size(), 0);

        let envelope = AABB::from_corners([0.0, 0.0], [100.0, 100.0]);
        let results: Vec<_> = tree.locate_in_envelope(&envelope).collect();
        assert!(results.is_empty());

        assert!(tree.nearest_neighbor(&[0.0, 0.0]).is_none());
    }

    #[test]
    fn test_edge_index_single_edge() {
        let edges = vec![IndexedEdge::new(Line::new([0.0, 0.0], [100.0, 100.0]), 0)];
        let tree = RTree::bulk_load(edges);
        assert_eq!(tree.size(), 1);

        // Envelope must fully contain the edge's bounding box
        let envelope = AABB::from_corners([-1.0, -1.0], [101.0, 101.0]);
        let results: Vec<u32> = tree.locate_in_envelope(&envelope).map(|e| e.data).collect();
        assert_eq!(results, vec![0]);

        let nearest = tree.nearest_neighbor(&[50.0, 50.0]).unwrap();
        assert_eq!(nearest.data, 0);
    }
}
