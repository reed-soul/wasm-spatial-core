//! OBJ (Wavefront) parser — point cloud mode.
//!
//! Extracts vertex positions (`v x y z`) and vertex normals (`vn nx ny nz`).
//! Face data is ignored (point cloud mode).

use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::DEFAULT_MAX_INPUT_SIZE;

// ===========================================================================
// Core Parsers (no WASM dependency, testable everywhere)
// ===========================================================================

/// Extract vertex positions from an OBJ file.
///
/// Only processes `v x y z` lines. Faces (`f`), materials (`mtl`), normals, etc.
/// are ignored.
///
/// Returns a flat Float32Array of [x0, y0, z0, x1, y1, z1, ...].
pub fn parse_obj_vertices_core(text: &str) -> Vec<f32> {
    let mut positions = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("v ") || line.starts_with("v\t") {
            let parts: Vec<&str> = line[2..].split_whitespace().collect();
            if parts.len() >= 3 {
                // OBJ uses float coordinates
                if let (Ok(x), Ok(y), Ok(z)) =
                    (parts[0].parse::<f32>(), parts[1].parse::<f32>(), parts[2].parse::<f32>())
                {
                    positions.push(x);
                    positions.push(y);
                    positions.push(z);
                }
            }
        }
    }
    positions
}

/// Extract vertex positions AND vertex normals from an OBJ file.
///
/// Processes `v x y z` and `vn nx ny nz` lines. Normals are matched to
/// vertices by order: first `v` gets first `vn`, second `v` gets second `vn`, etc.
///
/// If normals count doesn't match vertices count, normals will be `None`.
pub fn parse_obj_with_normals_core(text: &str) -> (Vec<f32>, Option<Vec<f32>>) {
    let positions = parse_obj_vertices_core(text);
    let mut normals_raw = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("vn ") || line.starts_with("vn\t") {
            let parts: Vec<&str> = line[3..].split_whitespace().collect();
            if parts.len() >= 3 {
                if let (Ok(nx), Ok(ny), Ok(nz)) =
                    (parts[0].parse::<f32>(), parts[1].parse::<f32>(), parts[2].parse::<f32>())
                {
                    normals_raw.push(nx);
                    normals_raw.push(ny);
                    normals_raw.push(nz);
                }
            }
        }
    }

    let vertex_count = positions.len() / 3;
    let normal_count = normals_raw.len() / 3;

    if normal_count == vertex_count && normal_count > 0 {
        (positions, Some(normals_raw))
    } else {
        (positions, None)
    }
}

// ===========================================================================
// WASM API
// ===========================================================================

/// Extract vertex positions from an OBJ file.
///
/// Returns a Float32Array of [x0, y0, z0, x1, y1, z1, ...].
/// Only processes `v` lines; faces, materials, etc. are ignored.
#[wasm_bindgen(js_name = "parseObjVertices")]
pub fn parse_obj_vertices(text: &str) -> js_sys::Float32Array {
    let positions = parse_obj_vertices_core(text);
    js_sys::Float32Array::from(&positions[..])
}

/// Extract vertex positions and normals from an OBJ file.
///
/// Returns a JS object: `{ positions: Float32Array, normals: Float32Array | null }`.
/// Normals are matched to vertices by order; returns null if counts don't match.
#[wasm_bindgen(js_name = "parseObjWithNormals")]
pub fn parse_obj_with_normals(text: &str) -> Result<js_sys::Object, SpatialErrorDetail> {
    if text.len() > DEFAULT_MAX_INPUT_SIZE {
        return Err(SpatialError::InputTooLarge.with_detail(format!(
            "OBJ input is {} bytes, max is {}",
            text.len(),
            DEFAULT_MAX_INPUT_SIZE
        )));
    }

    let (positions, normals) = parse_obj_with_normals_core(text);
    let obj = js_sys::Object::new();

    js_sys::Reflect::set(&obj, &"positions".into(), &js_sys::Float32Array::from(&positions[..])).unwrap();

    match normals {
        Some(n) => {
            js_sys::Reflect::set(&obj, &"normals".into(), &js_sys::Float32Array::from(&n[..])).unwrap();
        }
        None => {
            js_sys::Reflect::set(&obj, &"normals".into(), &JsValue::NULL).unwrap();
        }
    }

    Ok(obj)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_obj(positions: &[(f32, f32, f32)], normals: Option<&[(f32, f32, f32)]>, faces: bool) -> String {
        let mut s = String::new();
        // Optional comment
        s.push_str("# Test OBJ file\n");
        for &(x, y, z) in positions {
            s.push_str(&format!("v {} {} {}\n", x, y, z));
        }
        if let Some(nors) = normals {
            for &(nx, ny, nz) in nors {
                s.push_str(&format!("vn {} {} {}\n", nx, ny, nz));
            }
        }
        if faces && positions.len() >= 3 {
            // Simple face using vertex indices
            s.push_str(&format!(
                "f 1 2 3\n"
            ));
        }
        s
    }

    #[test]
    fn test_simple_vertices() {
        let obj = make_obj(&[(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)], None, false);
        let positions = parse_obj_vertices_core(&obj);
        assert_eq!(positions.len(), 6);
        assert_eq!(positions[0], 1.0);
        assert_eq!(positions[1], 2.0);
        assert_eq!(positions[2], 3.0);
        assert_eq!(positions[3], 4.0);
        assert_eq!(positions[4], 5.0);
        assert_eq!(positions[5], 6.0);
    }

    #[test]
    fn test_ignores_non_vertex_lines() {
        let obj = make_obj(&[(1.0, 2.0, 3.0)], None, true);
        let positions = parse_obj_vertices_core(&obj);
        // Should have 3 values (one vertex), face line ignored
        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0], 1.0);
    }

    #[test]
    fn test_with_matching_normals() {
        let pts = vec![(0.0, 0.0, 1.0), (1.0, 0.0, 0.0)];
        let nors = vec![(0.0, 0.0, 1.0), (1.0, 0.0, 0.0)];
        let obj = make_obj(&pts, Some(&nors), false);
        let (positions, normals) = parse_obj_with_normals_core(&obj);
        assert_eq!(positions.len(), 6);
        assert!(normals.is_some());
        let n = normals.unwrap();
        assert_eq!(n[0], 0.0);
        assert_eq!(n[2], 1.0);
        assert_eq!(n[3], 1.0);
    }

    #[test]
    fn test_mismatched_normals_returns_none() {
        let pts = vec![(0.0, 0.0, 1.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)];
        let nors = vec![(0.0, 0.0, 1.0)]; // Only 1 normal for 3 vertices
        let obj = make_obj(&pts, Some(&nors), false);
        let (_positions, normals) = parse_obj_with_normals_core(&obj);
        assert!(normals.is_none());
    }

    #[test]
    fn test_empty_obj() {
        let positions = parse_obj_vertices_core("");
        assert_eq!(positions.len(), 0);
    }

    #[test]
    fn test_negative_coordinates() {
        let obj = make_obj(&[(-1.0, -2.0, -3.0), (0.5, -0.5, 0.5)], None, false);
        let positions = parse_obj_vertices_core(&obj);
        assert_eq!(positions[0], -1.0);
        assert_eq!(positions[1], -2.0);
        assert_eq!(positions[2], -3.0);
        assert_eq!(positions[3], 0.5);
    }

    #[test]
    fn test_many_vertices() {
        let pts: Vec<(f32, f32, f32)> = (0..500)
            .map(|i| (i as f32, (i as f32).sin(), (i as f32).cos()))
            .collect();
        let mut obj = String::new();
        for &(x, y, z) in &pts {
            obj.push_str(&format!("v {} {} {}\n", x, y, z));
        }
        let positions = parse_obj_vertices_core(&obj);
        assert_eq!(positions.len() / 3, 500);
    }
}
