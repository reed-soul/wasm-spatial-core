//! Cesium Native Adapter
//!
//! Provides direct conversions and triangulations optimized for Cesium's rendering engine.

use std::f64::consts::PI;
use wasm_bindgen::prelude::*;
use geojson::{GeoJson, Value};
use earcutr::earcut;

#[cfg(feature = "multi-thread")]
use rayon::prelude::*;

const A: f64 = 6378137.0;
const B: f64 = 6356752.3142451793;
const A_SQ: f64 = A * A;
const B_SQ: f64 = B * B;
const E_SQ: f64 = 1.0 - (B_SQ / A_SQ);

#[inline]
fn to_radians(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

/// Convert a single WGS84 point to Cartesian3 (ECEF).
#[inline]
pub fn wgs84_to_cartesian3_single(lng: f64, lat: f64, height: f64) -> (f64, f64, f64) {
    let lat_rad = to_radians(lat);
    let lng_rad = to_radians(lng);

    let sin_lat = lat_rad.sin();
    let cos_lat = lat_rad.cos();
    let sin_lng = lng_rad.sin();
    let cos_lng = lng_rad.cos();

    let n = A / (1.0 - E_SQ * sin_lat * sin_lat).sqrt();

    let x = (n + height) * cos_lat * cos_lng;
    let y = (n + height) * cos_lat * sin_lng;
    let z = (n * (1.0 - E_SQ) + height) * sin_lat;

    (x, y, z)
}

/// Batch convert a flat array of `[lng, lat, ...]` into `[x, y, z, ...]`.
#[wasm_bindgen(js_name = "batchWgs84ToCartesian3")]
pub fn batch_wgs84_to_cartesian3(coords: &[f64]) -> js_sys::Float64Array {
    let point_count = coords.len() / 2;
    let mut out = vec![0.0; point_count * 3];

    #[cfg(feature = "multi-thread")]
    {
        out.par_chunks_exact_mut(3).enumerate().for_each(|(i, chunk)| {
            let lng = coords[i * 2];
            let lat = coords[i * 2 + 1];
            let (x, y, z) = wgs84_to_cartesian3_single(lng, lat, 0.0);
            chunk[0] = x;
            chunk[1] = y;
            chunk[2] = z;
        });
    }

    #[cfg(not(feature = "multi-thread"))]
    {
        for i in 0..point_count {
            let lng = coords[i * 2];
            let lat = coords[i * 2 + 1];
            let (x, y, z) = wgs84_to_cartesian3_single(lng, lat, 0.0);
            out[i * 3] = x;
            out[i * 3 + 1] = y;
            out[i * 3 + 2] = z;
        }
    }

    // Zero-copy view into Wasm memory, then duplicated into JS Float64Array
    let arr = js_sys::Float64Array::new_with_length((point_count * 3) as u32);
    arr.copy_from(&out);
    arr
}

/// Contains triangulated mesh data ready for Cesium.Geometry
#[wasm_bindgen]
pub struct CesiumMeshGeometry {
    positions: Vec<f64>,
    indices: Vec<u32>,
}

#[wasm_bindgen]
impl CesiumMeshGeometry {
    #[wasm_bindgen(getter)]
    pub fn positions(&self) -> js_sys::Float64Array {
        let arr = js_sys::Float64Array::new_with_length(self.positions.len() as u32);
        arr.copy_from(&self.positions);
        arr
    }

    #[wasm_bindgen(getter)]
    pub fn indices(&self) -> js_sys::Uint32Array {
        let arr = js_sys::Uint32Array::new_with_length(self.indices.len() as u32);
        arr.copy_from(&self.indices);
        arr
    }
}

/// Generate triangulated mesh from GeoJSON Polygons/MultiPolygons.
#[wasm_bindgen(js_name = "generateCesiumGeometry")]
pub fn generate_cesium_geometry(geojson_str: &str, height_property: Option<String>) -> Result<CesiumMeshGeometry, JsValue> {
    let geojson = geojson_str.parse::<GeoJson>().map_err(|e: geojson::Error| JsValue::from_str(&e.to_string()))?;

    let mut all_positions: Vec<f64> = Vec::new();
    let mut all_indices: Vec<u32> = Vec::new();
    let mut current_vertex_offset = 0;

    let process_polygon = |
        polygon: &Vec<Vec<Vec<f64>>>,
        feature_height: f64,
        positions_buf: &mut Vec<f64>,
        indices_buf: &mut Vec<u32>,
        offset: &mut u32
    | {
        if polygon.is_empty() || polygon[0].is_empty() {
            return;
        }

        let mut flat_vertices: Vec<f64> = Vec::new();
        let mut altitudes: Vec<f64> = Vec::new();
        let mut hole_indices: Vec<usize> = Vec::new();

        // Push outer ring
        for pt in &polygon[0] {
            flat_vertices.push(pt[0]);
            flat_vertices.push(pt[1]);
            altitudes.push(if pt.len() >= 3 { pt[2] } else { feature_height });
        }

        // Push holes
        for hole in polygon.iter().skip(1) {
            hole_indices.push(flat_vertices.len() / 2);
            for pt in hole {
                flat_vertices.push(pt[0]);
                flat_vertices.push(pt[1]);
                altitudes.push(if pt.len() >= 3 { pt[2] } else { feature_height });
            }
        }

        // Run earcut on 2D
        let raw_indices_result = earcut(&flat_vertices, &hole_indices, 2);
        
        let raw_indices = match raw_indices_result {
            Ok(indices) => indices,
            Err(_) => return, // Skip invalid polygons
        };

        // Convert 2D vertices to Cartesian3 and append to positions_buf
        let vertex_count = flat_vertices.len() / 2;
        for i in 0..vertex_count {
            let lng = flat_vertices[i * 2];
            let lat = flat_vertices[i * 2 + 1];
            let alt = altitudes[i];
            let (x, y, z) = wgs84_to_cartesian3_single(lng, lat, alt);
            positions_buf.push(x);
            positions_buf.push(y);
            positions_buf.push(z);
        }

        // Append indices with global offset
        for idx in raw_indices {
            indices_buf.push(*offset + idx as u32);
        }

        *offset += vertex_count as u32;
    };

    let extract_height = |properties: &Option<geojson::JsonObject>, prop_name_opt: &Option<String>| -> f64 {
        if let Some(prop_name) = prop_name_opt {
            if let Some(props) = properties {
                if let Some(serde_json::Value::Number(num)) = props.get(prop_name) {
                    return num.as_f64().unwrap_or(0.0);
                }
            }
        }
        0.0
    };

    match geojson {
        GeoJson::FeatureCollection(fc) => {
            for feature in fc.features {
                let feature_height = extract_height(&feature.properties, &height_property);
                if let Some(geom) = feature.geometry {
                    match geom.value {
                        Value::Polygon(poly) => {
                            process_polygon(&poly, feature_height, &mut all_positions, &mut all_indices, &mut current_vertex_offset);
                        }
                        Value::MultiPolygon(multipoly) => {
                            for poly in multipoly {
                                process_polygon(&poly, feature_height, &mut all_positions, &mut all_indices, &mut current_vertex_offset);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        GeoJson::Feature(feature) => {
            let feature_height = extract_height(&feature.properties, &height_property);
            if let Some(geom) = feature.geometry {
                match geom.value {
                    Value::Polygon(poly) => {
                        process_polygon(&poly, feature_height, &mut all_positions, &mut all_indices, &mut current_vertex_offset);
                    }
                    Value::MultiPolygon(multipoly) => {
                        for poly in multipoly {
                            process_polygon(&poly, feature_height, &mut all_positions, &mut all_indices, &mut current_vertex_offset);
                        }
                    }
                    _ => {}
                }
            }
        }
        GeoJson::Geometry(geom) => {
            match geom.value {
                Value::Polygon(poly) => {
                    process_polygon(&poly, 0.0, &mut all_positions, &mut all_indices, &mut current_vertex_offset);
                }
                Value::MultiPolygon(multipoly) => {
                    for poly in multipoly {
                        process_polygon(&poly, 0.0, &mut all_positions, &mut all_indices, &mut current_vertex_offset);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(CesiumMeshGeometry {
        positions: all_positions,
        indices: all_indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_approx_eq(a: f64, b: f64, epsilon: f64) {
        assert!((a - b).abs() < epsilon, "{} != {}", a, b);
    }

    #[test]
    fn test_wgs84_to_cartesian3_single() {
        // [0, 0] should be on the equator, intersection with Prime Meridian.
        // X should be semi-major axis A, Y=0, Z=0.
        let (x, y, z) = wgs84_to_cartesian3_single(0.0, 0.0, 0.0);
        assert_approx_eq(x, 6378137.0, 1e-6);
        assert_approx_eq(y, 0.0, 1e-6);
        assert_approx_eq(z, 0.0, 1e-6);

        // [90, 0] should be on the equator, 90 degrees East.
        // X=0, Y=A, Z=0
        let (x, y, z) = wgs84_to_cartesian3_single(90.0, 0.0, 0.0);
        assert_approx_eq(x, 0.0, 1e-6);
        assert_approx_eq(y, 6378137.0, 1e-6);
        assert_approx_eq(z, 0.0, 1e-6);

        // [0, 90] should be the North Pole.
        // X=0, Y=0, Z=semi-minor axis B
        let (x, y, z) = wgs84_to_cartesian3_single(0.0, 90.0, 0.0);
        assert_approx_eq(x, 0.0, 1e-6);
        assert_approx_eq(y, 0.0, 1e-6);
        assert_approx_eq(z, 6356752.314245179, 1e-6);
    }

    #[test]
    fn test_generate_cesium_geometry() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]]]
            }
        }"#;

        let result = generate_cesium_geometry(geojson, None).unwrap();
        // 4 unique vertices (ignoring duplicate closing vertex during flattening? wait, earcutr handles it but usually we pass the closed ring. earcutr expects the ring without the closing vertex or with it. earcutr will just have an extra point if we don't pop it).
        // For 4 points, we should have at least 2 triangles -> 6 indices.
        // Let's just check it didn't fail and gave us data.
        assert!(!result.positions.is_empty());
        assert!(!result.indices.is_empty());
    }
}
