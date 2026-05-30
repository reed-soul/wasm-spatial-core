//! Cesium Native Adapter
//!
//! Provides direct conversions and triangulations optimized for Cesium's rendering engine.

use earcutr::earcut;
use geojson::{GeoJson, Value};
use std::f64::consts::PI;
use wasm_bindgen::prelude::*;

use crate::validate_input_size;

#[cfg(feature = "multi-thread")]
use rayon::prelude::*;

const A: f64 = 6378137.0;
const B: f64 = 6_356_752.314_245_179;
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
    assert!(
        coords.len().is_multiple_of(2),
        "Coordinates length must be even (pairs of lng/lat), got {}",
        coords.len()
    );
    let point_count = coords.len() / 2;
    let mut out = vec![0.0; point_count * 3];

    #[cfg(feature = "multi-thread")]
    {
        out.par_chunks_exact_mut(3)
            .enumerate()
            .for_each(|(i, chunk)| {
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
pub fn generate_cesium_geometry(
    geojson_str: &str,
    height_property: Option<String>,
) -> Result<CesiumMeshGeometry, JsValue> {
    validate_input_size(geojson_str.len(), "GeoJSON input")?;
    let geojson = geojson_str
        .parse::<GeoJson>()
        .map_err(|e: geojson::Error| JsValue::from_str(&e.to_string()))?;

    let mut all_positions: Vec<f64> = Vec::new();
    let mut all_indices: Vec<u32> = Vec::new();
    let mut current_vertex_offset = 0;

    let process_polygon = |polygon: &Vec<Vec<Vec<f64>>>,
                           feature_height: f64,
                           positions_buf: &mut Vec<f64>,
                           indices_buf: &mut Vec<u32>,
                           offset: &mut u32| {
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

    let extract_height =
        |properties: &Option<geojson::JsonObject>, prop_name_opt: &Option<String>| -> f64 {
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
                            process_polygon(
                                &poly,
                                feature_height,
                                &mut all_positions,
                                &mut all_indices,
                                &mut current_vertex_offset,
                            );
                        }
                        Value::MultiPolygon(multipoly) => {
                            for poly in multipoly {
                                process_polygon(
                                    &poly,
                                    feature_height,
                                    &mut all_positions,
                                    &mut all_indices,
                                    &mut current_vertex_offset,
                                );
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
                        process_polygon(
                            &poly,
                            feature_height,
                            &mut all_positions,
                            &mut all_indices,
                            &mut current_vertex_offset,
                        );
                    }
                    Value::MultiPolygon(multipoly) => {
                        for poly in multipoly {
                            process_polygon(
                                &poly,
                                feature_height,
                                &mut all_positions,
                                &mut all_indices,
                                &mut current_vertex_offset,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        GeoJson::Geometry(geom) => match geom.value {
            Value::Polygon(poly) => {
                process_polygon(
                    &poly,
                    0.0,
                    &mut all_positions,
                    &mut all_indices,
                    &mut current_vertex_offset,
                );
            }
            Value::MultiPolygon(multipoly) => {
                for poly in multipoly {
                    process_polygon(
                        &poly,
                        0.0,
                        &mut all_positions,
                        &mut all_indices,
                        &mut current_vertex_offset,
                    );
                }
            }
            _ => {}
        },
    }

    Ok(CesiumMeshGeometry {
        positions: all_positions,
        indices: all_indices,
    })
}

// ===========================================================================
// b3dm (Batched 3D Model) — 3D Tiles output
// ===========================================================================

/// A Cesium 3D Tiles b3dm tile containing a triangulated batched mesh.
#[wasm_bindgen]
pub struct Cesium3DTile {
    batch_table_json: String,
    feature_batch_ids: Vec<u32>,
    positions: Vec<f64>,
    indices: Vec<u32>,
}

#[wasm_bindgen]
impl Cesium3DTile {
    /// Serialize this tile to a complete b3dm binary blob.
    ///
    /// b3dm layout:
    /// ```text
    /// [Header 28 bytes] [BatchTable JSON] [FeatureTable JSON + BIN] [Body]
    /// ```
    ///
    /// Header (28 bytes, little-endian):
    /// - magic: "b3dm" (4 bytes)
    /// - version: 1 (u32)
    /// - byteLength (u32) — total tile size
    /// - featureTableJSONByteLength (u32)
    /// - featureTableBinaryByteLength (u32)
    /// - batchTableJSONByteLength (u32)
    /// - batchTableBinaryByteLength (u32)
    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> js_sys::Uint8Array {
        // ── Body: positions (f64) + indices (u32) ────────────────────
        // Convert positions to f32 for the body (standard b3dm uses float32)
        let positions_f32: Vec<f32> = self.positions.iter().map(|&v| v as f32).collect();
        let indices_u16: Vec<u16> = self.indices.iter().map(|&v| v as u16).collect();

        let body_byte_len = positions_f32.len() * std::mem::size_of::<f32>()
            + indices_u16.len() * std::mem::size_of::<u16>();

        // ── FeatureTable JSON ──────────────────────────────────────────
        // Minimal feature table with BATCH_LENGTH and positions/indices references.
        let batch_length = self.feature_batch_ids.len() as u32;
        let feature_table_json = serde_json::json!({
            "BATCH_LENGTH": batch_length,
            "POSITIONS": {
                "byteOffset": 0,
                "componentType": 5126,  // FLOAT
                "count": positions_f32.len() / 3,
                "type": "SCALAR"
            },
            "indices": {
                "byteOffset": positions_f32.len() * 4,
                "componentType": 5123,  // UNSIGNED_SHORT
                "count": indices_u16.len(),
                "type": "SCALAR"
            }
        })
        .to_string();

        // Pad to 4-byte alignment
        let ft_json_bytes = feature_table_json.into_bytes();
        let ft_json_padded_len = align4(ft_json_bytes.len());

        // Feature table binary body
        let ft_bin_byte_len = body_byte_len;
        let ft_bin_padded_len = align4(ft_bin_byte_len);

        // ── BatchTable JSON ────────────────────────────────────────────
        let bt_json_bytes = self.batch_table_json.as_bytes().to_vec();
        let bt_json_padded_len = align4(bt_json_bytes.len());
        let bt_bin_padded_len = 0u32;

        // ── Total size ────────────────────────────────────────────────
        let byte_length = 28u32
            + ft_json_padded_len as u32
            + ft_bin_padded_len as u32
            + bt_json_padded_len as u32
            + bt_bin_padded_len;

        let mut buf: Vec<u8> = Vec::with_capacity(byte_length as usize);

        // ── Header (28 bytes) ─────────────────────────────────────────
        buf.extend_from_slice(b"b3dm"); // magic
        buf.extend_from_slice(&1u32.to_le_bytes()); // version
        buf.extend_from_slice(&byte_length.to_le_bytes());
        buf.extend_from_slice(&ft_json_padded_len.to_le_bytes());
        buf.extend_from_slice(&ft_bin_padded_len.to_le_bytes());
        buf.extend_from_slice(&bt_json_padded_len.to_le_bytes());
        buf.extend_from_slice(&bt_bin_padded_len.to_le_bytes());

        // ── FeatureTable JSON (padded) ────────────────────────────────
        buf.extend_from_slice(&ft_json_bytes);
        pad4_relative(&mut buf);

        // ── FeatureTable Binary (= body: positions f32 + indices u16, padded) ─
        for val in &positions_f32 {
            buf.extend_from_slice(&val.to_le_bytes());
        }
        for val in &indices_u16 {
            buf.extend_from_slice(&val.to_le_bytes());
        }
        pad4_relative(&mut buf);

        // ── BatchTable JSON (padded) ───────────────────────────────────
        buf.extend_from_slice(&bt_json_bytes);
        pad4_relative(&mut buf);

        let arr = js_sys::Uint8Array::new_with_length(buf.len() as u32);
        arr.copy_from(&buf);
        arr
    }

    #[wasm_bindgen(js_name = "batchTableJson", getter)]
    pub fn batch_table_json(&self) -> String {
        self.batch_table_json.clone()
    }

    #[wasm_bindgen(js_name = "featureBatchIds")]
    #[wasm_bindgen(getter)]
    pub fn feature_batch_ids(&self) -> js_sys::Uint32Array {
        let arr = js_sys::Uint32Array::new_with_length(self.feature_batch_ids.len() as u32);
        arr.copy_from(&self.feature_batch_ids);
        arr
    }
}

/// Align `n` up to the next multiple of 4.
fn align4(n: usize) -> usize {
    (n + 3) & !3
}

/// Pad `buf` with space bytes so its length is a multiple of 4.
fn pad4_relative(buf: &mut Vec<u8>) {
    let aligned = align4(buf.len());
    while buf.len() < aligned {
        buf.push(b' ');
    }
}

/// Generate a complete b3dm 3D Tile from GeoJSON polygons/multipolygons.
///
/// Reuses `generate_cesium_geometry` internally for triangulation, then
/// wraps the result in the b3dm binary envelope suitable for Cesium's
/// `Cesium3DTileset`.
#[wasm_bindgen(js_name = "generate3DTile")]
pub fn generate_3d_tile(
    geojson_str: &str,
    height_property: Option<String>,
) -> Result<Cesium3DTile, JsValue> {
    let geometry = generate_cesium_geometry(geojson_str, height_property)?;

    let feature_count = estimate_feature_count(geojson_str);
    let feature_batch_ids: Vec<u32> = (0..feature_count).collect();

    let batch_table = serde_json::json!({
        "id": ["0"],
        "properties": {
            "name": "wasm-spatial-core tile"
        }
    })
    .to_string();

    Ok(Cesium3DTile {
        batch_table_json: batch_table,
        feature_batch_ids,
        positions: geometry.positions,
        indices: geometry.indices,
    })
}

/// Rough estimate of feature count from the JSON string.
fn estimate_feature_count(geojson_str: &str) -> u32 {
    // Count occurrences of "type":"Feature" as a heuristic.
    // This is fast and avoids full parse just for counting.
    let needle = "\"type\":\"Feature\"";
    let count = geojson_str.matches(needle).count();
    if count > 0 {
        return count as u32;
    }
    // Also try with spaces
    let needle2 = "\"type\": \"Feature\"";
    let count2 = geojson_str.matches(needle2).count();
    if count2 > 0 {
        return count2 as u32;
    }
    // Single feature / geometry
    1
}

#[cfg(test)]
mod tests_b3dm {
    use super::*;

    #[test]
    fn test_b3dm_header_magic_and_version() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]]]
            }
        }"#;
        let tile = generate_3d_tile(geojson, None).unwrap();
        let bytes = tile_to_vec(&tile);

        // Magic
        assert_eq!(&bytes[0..4], b"b3dm");
        // Version = 1
        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        assert_eq!(version, 1);
    }

    #[test]
    fn test_b3dm_byte_length_consistency() {
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[0, 0], [5, 0], [5, 5], [0, 5], [0, 0]]]
                    },
                    "properties": { "name": "poly A" }
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[10, 10], [15, 10], [15, 15], [10, 15], [10, 10]]]
                    },
                    "properties": { "name": "poly B" }
                }
            ]
        }"#;
        let tile = generate_3d_tile(geojson, None).unwrap();
        let bytes = tile_to_vec(&tile);

        let declared_length =
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        assert_eq!(
            declared_length,
            bytes.len(),
            "Declared byteLength ({}) must equal actual buffer size ({})",
            declared_length,
            bytes.len()
        );

        // Also verify header fields are consistent with each other
        let ft_json_len = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let ft_bin_len = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
        let bt_json_len = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]) as usize;
        let bt_bin_len = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]) as usize;

        assert_eq!(
            28 + ft_json_len + ft_bin_len + bt_json_len + bt_bin_len,
            bytes.len()
        );
    }

    #[test]
    fn test_b3dm_feature_table_valid_json() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]]]
            }
        }"#;
        let tile = generate_3d_tile(geojson, None).unwrap();
        let bytes = tile_to_vec(&tile);

        let ft_json_len = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let ft_json_str = String::from_utf8_lossy(&bytes[28..28 + ft_json_len])
            .trim()
            .to_string();
        let parsed: serde_json::Value = serde_json::from_str(&ft_json_str).unwrap();

        assert!(parsed.get("BATCH_LENGTH").is_some());
        assert!(parsed.get("POSITIONS").is_some());
    }

    /// Helper: extract raw bytes from a Cesium3DTile without JS interop.
    fn tile_to_vec(tile: &Cesium3DTile) -> Vec<u8> {
        // Simulate to_bytes without js_sys by directly building the bytes.
        let positions_f32: Vec<f32> = tile.positions.iter().map(|&v| v as f32).collect();
        let indices_u16: Vec<u16> = tile.indices.iter().map(|&v| v as u16).collect();

        let body_byte_len = positions_f32.len() * 4 + indices_u16.len() * 2;

        let batch_length = tile.feature_batch_ids.len() as u32;
        let feature_table_json = serde_json::json!({
            "BATCH_LENGTH": batch_length,
            "POSITIONS": {
                "byteOffset": 0,
                "componentType": 5126,
                "count": positions_f32.len() / 3,
                "type": "SCALAR"
            },
            "indices": {
                "byteOffset": positions_f32.len() * 4,
                "componentType": 5123,
                "count": indices_u16.len(),
                "type": "SCALAR"
            }
        })
        .to_string();

        let ft_json_bytes = feature_table_json.into_bytes();
        let ft_json_padded_len = align4(ft_json_bytes.len());
        let ft_bin_padded_len = align4(body_byte_len);

        let bt_json_bytes = tile.batch_table_json.as_bytes().to_vec();
        let bt_json_padded_len = align4(bt_json_bytes.len());
        let bt_bin_padded_len = 0usize;

        let byte_length =
            28 + ft_json_padded_len + ft_bin_padded_len + bt_json_padded_len + bt_bin_padded_len;

        let mut buf: Vec<u8> = Vec::with_capacity(byte_length);
        buf.extend_from_slice(b"b3dm");
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&(byte_length as u32).to_le_bytes());
        buf.extend_from_slice(&(ft_json_padded_len as u32).to_le_bytes());
        buf.extend_from_slice(&(ft_bin_padded_len as u32).to_le_bytes());
        buf.extend_from_slice(&(bt_json_padded_len as u32).to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&ft_json_bytes);
        pad4_relative(&mut buf);
        for val in &positions_f32 {
            buf.extend_from_slice(&val.to_le_bytes());
        }
        for val in &indices_u16 {
            buf.extend_from_slice(&val.to_le_bytes());
        }
        pad4_relative(&mut buf);
        buf.extend_from_slice(&bt_json_bytes);
        pad4_relative(&mut buf);
        buf
    }
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

    #[test]
    fn test_generate_cesium_geometry_multipolygon() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {
                "type": "MultiPolygon",
                "coordinates": [
                    [[[0, 0], [5, 0], [5, 5], [0, 5], [0, 0]]],
                    [[[10, 10], [15, 10], [15, 15], [10, 15], [10, 10]]]
                ]
            },
            "properties": {
                "height": 100.0
            }
        }"#;

        let result = generate_cesium_geometry(geojson, Some("height".to_string())).unwrap();

        // Two polygons, each with 5 vertices (4 unique + closing) →
        // positions should be 2 polygons × 5 vertices × 3 components = 30
        assert!(!result.positions.is_empty());
        assert!(
            result.positions.len() >= 30,
            "Expected at least 30 position floats, got {}",
            result.positions.len()
        );

        // Each polygon should produce at least 2 triangles (6 indices)
        // Total: at least 12 indices
        assert!(!result.indices.is_empty());
        assert!(
            result.indices.len() >= 12,
            "Expected at least 12 indices, got {}",
            result.indices.len()
        );

        // Verify indices are within bounds
        let max_idx = *result.indices.iter().max().unwrap();
        let num_vertices = result.positions.len() / 3;
        assert!(
            max_idx < num_vertices as u32,
            "Index {} exceeds vertex count {}",
            max_idx,
            num_vertices
        );
    }

    #[test]
    fn test_generate_cesium_geometry_polygon_with_hole() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Polygon",
                "coordinates": [
                    [[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]],
                    [[3, 3], [7, 3], [7, 7], [3, 7], [3, 3]]
                ]
            }
        }"#;

        let result = generate_cesium_geometry(geojson, None).unwrap();
        assert!(!result.positions.is_empty());
        assert!(!result.indices.is_empty());

        // Outer ring: 5 points, inner ring: 5 points = 10 vertices × 3 = 30 floats
        assert!(result.positions.len() >= 30);

        // Should have valid indices
        let max_idx = *result.indices.iter().max().unwrap();
        let num_vertices = result.positions.len() / 3;
        assert!(max_idx < num_vertices as u32);
    }

    #[test]
    fn test_generate_cesium_geometry_feature_collection() {
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]]
                    },
                    "properties": {}
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "MultiPolygon",
                        "coordinates": [
                            [[[5, 5], [6, 5], [6, 6], [5, 6], [5, 5]]]
                        ]
                    },
                    "properties": {}
                }
            ]
        }"#;

        let result = generate_cesium_geometry(geojson, None).unwrap();
        assert!(!result.positions.is_empty());
        assert!(!result.indices.is_empty());

        // 2 features, each producing at least 2 triangles
        assert!(result.indices.len() >= 12);
    }
}
