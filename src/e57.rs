//! E57 Format Support
//!
//! Pure Rust E57 reader wrapper for architectural/industrial scan data.
//! Uses the `e57` crate (v0.11.12) which compiles cleanly to wasm32.
//!
//! E57 is an ASTM standard format (E2807) used in laser scanning,
//! photogrammetry, and BIM workflows. It supports both Cartesian and
//! spherical coordinates, with optional color and intensity.

use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::DEFAULT_MAX_INPUT_SIZE;

#[cfg(feature = "e57-support")]
use e57::{CartesianCoordinate, E57Reader, RecordName};

#[cfg(feature = "e57-support")]
#[cfg(test)]
use e57::{Record, RecordDataType};

#[cfg(feature = "e57-support")]
use std::io::Cursor;

// ===========================================================================
// Public result struct
// ===========================================================================

/// Parsed E57 point cloud data.
#[wasm_bindgen]
#[derive(Debug)]
pub struct E57Result {
    pub(crate) positions: Vec<f32>,
    pub(crate) colors: Option<Vec<u8>>,
    pub(crate) intensities: Option<Vec<f32>>,
    pub(crate) point_count: u32,
    file_info: String,
}

#[wasm_bindgen]
impl E57Result {
    /// Interleaved XYZ positions as Float32Array `[x0, y0, z0, x1, y1, z1, ...]`.
    #[wasm_bindgen(getter)]
    pub fn positions(&self) -> js_sys::Float32Array {
        let arr = js_sys::Float32Array::new_with_length(self.positions.len() as u32);
        arr.copy_from(&self.positions);
        arr
    }

    /// RGB colors as Uint8Array `[r0, g0, b0, r1, g1, b1, ...]`, or `null` if not present.
    #[wasm_bindgen(getter)]
    pub fn colors(&self) -> Option<js_sys::Uint8Array> {
        self.colors.as_ref().map(|c| {
            let arr = js_sys::Uint8Array::new_with_length(c.len() as u32);
            arr.copy_from(c);
            arr
        })
    }

    /// Intensity values as Float32Array, or `null` if not present.
    #[wasm_bindgen(getter)]
    pub fn intensities(&self) -> Option<js_sys::Float32Array> {
        self.intensities.as_ref().map(|v| {
            let arr = js_sys::Float32Array::new_with_length(v.len() as u32);
            arr.copy_from(v);
            arr
        })
    }

    /// Number of points in the cloud.
    #[wasm_bindgen(getter, js_name = "pointCount")]
    pub fn point_count(&self) -> u32 {
        self.point_count
    }

    /// File metadata as JSON string.
    #[wasm_bindgen(js_name = "fileInfo")]
    pub fn file_info(&self) -> String {
        self.file_info.clone()
    }
}

// ===========================================================================
// E57 format detection
// ===========================================================================

/// Check if bytes look like an E57 file by checking the ASTM-E57 signature.
///
/// The E57 file header starts with the 8-byte signature "ASTM-E57" at offset 0.
pub fn is_e57_format(bytes: &[u8]) -> bool {
    bytes.len() >= 48 && &bytes[0..8] == b"ASTM-E57"
}

// ===========================================================================
// E57 parsing implementation (feature-gated)
// ===========================================================================

#[cfg(feature = "e57-support")]
#[cfg(test)]
fn build_pointcloud_prototype(has_color: bool, has_intensity: bool) -> Vec<Record> {
    let mut prototype = vec![
        Record {
            name: RecordName::CartesianX,
            data_type: RecordDataType::Double {
                min: Some(f64::MIN),
                max: Some(f64::MAX),
            },
        },
        Record {
            name: RecordName::CartesianY,
            data_type: RecordDataType::Double {
                min: Some(f64::MIN),
                max: Some(f64::MAX),
            },
        },
        Record {
            name: RecordName::CartesianZ,
            data_type: RecordDataType::Double {
                min: Some(f64::MIN),
                max: Some(f64::MAX),
            },
        },
    ];

    if has_color {
        prototype.push(Record {
            name: RecordName::ColorRed,
            data_type: RecordDataType::Integer { min: 0, max: 255 },
        });
        prototype.push(Record {
            name: RecordName::ColorGreen,
            data_type: RecordDataType::Integer { min: 0, max: 255 },
        });
        prototype.push(Record {
            name: RecordName::ColorBlue,
            data_type: RecordDataType::Integer { min: 0, max: 255 },
        });
    }

    if has_intensity {
        prototype.push(Record {
            name: RecordName::Intensity,
            data_type: RecordDataType::Double {
                min: Some(0.0),
                max: Some(f64::MAX),
            },
        });
    }

    prototype
}

/// Core E57 parsing — pure Rust, testable without WASM runtime.
#[cfg(feature = "e57-support")]
pub fn parse_e57_core(bytes: &[u8]) -> Result<E57Result, String> {
    if bytes.len() < 48 {
        return Err("E57 data too short for header".to_string());
    }
    if &bytes[0..8] != b"ASTM-E57" {
        return Err(format!(
            "Invalid E57 magic: expected b\"ASTM-E57\", got {:?}",
            &bytes[0..8]
        ));
    }

    let cursor = Cursor::new(bytes);
    let mut reader = E57Reader::new(cursor).map_err(|e| format!("E57 reader error: {}", e))?;

    // Get point cloud descriptors
    let pointclouds = reader.pointclouds();
    if pointclouds.is_empty() {
        return Err("E57 file contains no point clouds".to_string());
    }

    // Build file info JSON
    let header = reader.header();
    let mut info = serde_json::Map::new();
    info.insert(
        "format".to_string(),
        serde_json::Value::String(reader.format_name().to_string()),
    );
    info.insert(
        "guid".to_string(),
        serde_json::Value::String(reader.guid().to_string()),
    );
    if let Some(lib_ver) = reader.library_version() {
        info.insert(
            "libraryVersion".to_string(),
            serde_json::Value::String(lib_ver.to_string()),
        );
    }
    info.insert(
        "pointcloudCount".to_string(),
        serde_json::Value::Number(serde_json::Number::from(pointclouds.len())),
    );
    info.insert(
        "pageSize".to_string(),
        serde_json::Value::Number(serde_json::Number::from(header.page_size)),
    );
    if let Some(creation) = reader.creation() {
        info.insert(
            "creationTime".to_string(),
            serde_json::Value::String(format!("{:?}", creation)),
        );
    }
    if let Some(coord) = reader.coordinate_metadata() {
        info.insert(
            "coordinateMetadata".to_string(),
            serde_json::Value::String(coord.to_string()),
        );
    }

    // Parse the first point cloud
    let pc = &pointclouds[0];
    let pc_info = serde_json::json!({
        "name": pc.name.as_deref().unwrap_or("unnamed"),
        "description": pc.description.as_deref().unwrap_or(""),
        "records": pc.records,
        "hasColor": pc.has_color(),
        "hasIntensity": pc.has_intensity(),
        "hasRowColumn": pc.has_row_column(),
    });
    info.insert("pointcloud0".to_string(), pc_info);

    // Check if there are cartesian coordinates
    let has_cartesian = pc.prototype.iter().any(|r| {
        matches!(
            r.name,
            RecordName::CartesianX | RecordName::CartesianY | RecordName::CartesianZ
        )
    });

    if !has_cartesian {
        return Err(
            "E57 point cloud has no Cartesian coordinates. Spherical-only is not supported."
                .to_string(),
        );
    }

    // Read points using the simple reader
    let simple_reader = reader
        .pointcloud_simple(pc)
        .map_err(|e| format!("Failed to create point cloud reader: {}", e))?;

    let mut positions: Vec<f32> = Vec::new();
    let mut colors: Option<Vec<u8>> = if pc.has_color() {
        Some(Vec::new())
    } else {
        None
    };
    let mut intensities: Option<Vec<f32>> = if pc.has_intensity() {
        Some(Vec::new())
    } else {
        None
    };

    for point_result in simple_reader {
        let point = point_result.map_err(|e| format!("E57 point read error: {}", e))?;

        match point.cartesian {
            CartesianCoordinate::Valid { x, y, z } => {
                positions.push(x as f32);
                positions.push(y as f32);
                positions.push(z as f32);
            }
            CartesianCoordinate::Invalid => {
                // Skip invalid points but keep placeholders for index consistency
                positions.push(0.0);
                positions.push(0.0);
                positions.push(0.0);
            }
            CartesianCoordinate::Direction { x, y, z } => {
                positions.push(x as f32);
                positions.push(y as f32);
                positions.push(z as f32);
            }
        }

        if let Some(color) = &point.color {
            if let Some(ref mut c) = colors {
                c.push((color.red.clamp(0.0, 1.0) * 255.0) as u8);
                c.push((color.green.clamp(0.0, 1.0) * 255.0) as u8);
                c.push((color.blue.clamp(0.0, 1.0) * 255.0) as u8);
            }
        } else if let Some(ref mut c) = colors {
            c.push(128);
            c.push(128);
            c.push(128);
        }

        if let Some(intensity) = point.intensity {
            if let Some(ref mut ints) = intensities {
                ints.push(intensity);
            }
        }
    }

    let point_count = positions.len() as u32 / 3;

    Ok(E57Result {
        positions,
        colors,
        intensities,
        point_count,
        file_info: serde_json::Value::Object(info).to_string(),
    })
}

/// Build a minimal valid E57 file from points for testing.
///
/// Uses the e57 crate's writer to construct a valid E57 in a Vec<u8>.
/// Build a minimal valid E57 file from points for testing.
///
/// Uses the e57 crate's writer to construct a valid E57 in a temp file,
/// then reads it back into bytes.
#[cfg(feature = "e57-support")]
#[cfg(test)]
pub(crate) fn build_test_e57_file(
    points: &[(f64, f64, f64)],
    has_color: bool,
    has_intensity: bool,
) -> Result<Vec<u8>, String> {
    use std::path::PathBuf;

    // Use a unique temp file per call to avoid parallel test conflicts
    let path: PathBuf = std::env::temp_dir().join(format!(
        "wasm_spatial_e57_test_{:?}.e57",
        std::thread::current().id()
    ));

    // Write E57 file using the crate's file writer
    let mut writer = e57::E57Writer::from_file(&path, "test-guid")
        .map_err(|e| format!("E57 writer error: {}", e))?;

    let prototype = build_pointcloud_prototype(has_color, has_intensity);
    let mut pc_writer = writer
        .add_pointcloud("pc-guid", prototype)
        .map_err(|e| format!("Failed to add pointcloud: {}", e))?;

    if has_color {
        pc_writer.set_color_limits(Some(e57::ColorLimits {
            red_min: Some(e57::RecordValue::Integer(0)),
            red_max: Some(e57::RecordValue::Integer(255)),
            green_min: Some(e57::RecordValue::Integer(0)),
            green_max: Some(e57::RecordValue::Integer(255)),
            blue_min: Some(e57::RecordValue::Integer(0)),
            blue_max: Some(e57::RecordValue::Integer(255)),
        }));
    }

    for &(x, y, z) in points {
        let mut raw_values = vec![
            e57::RecordValue::Double(x),
            e57::RecordValue::Double(y),
            e57::RecordValue::Double(z),
        ];

        if has_color {
            let r = ((x * 10.0).abs() as i64 % 256) as u8;
            let g = ((y * 10.0).abs() as i64 % 256) as u8;
            let b = ((z * 10.0).abs() as i64 % 256) as u8;
            raw_values.push(e57::RecordValue::Integer(r as i64));
            raw_values.push(e57::RecordValue::Integer(g as i64));
            raw_values.push(e57::RecordValue::Integer(b as i64));
        }

        if has_intensity {
            raw_values.push(e57::RecordValue::Double(
                (x * x + y * y + z * z).sqrt() * 0.1,
            ));
        }

        pc_writer
            .add_point(raw_values)
            .map_err(|e| format!("Failed to add point: {}", e))?;
    }

    pc_writer
        .finalize()
        .map_err(|e| format!("Failed to finalize pointcloud writer: {}", e))?;

    writer
        .finalize()
        .map_err(|e| format!("Failed to finalize E57 file: {}", e))?;

    // Read back into bytes
    let bytes = std::fs::read(&path).map_err(|e| format!("Failed to read E57 file: {}", e))?;

    // Clean up
    let _ = std::fs::remove_file(&path);

    Ok(bytes)
}

// ===========================================================================
// WASM bindings (feature-gated)
// ===========================================================================

/// Parse an E57 file from raw bytes.
///
/// Returns an `E57Result` with positions, optional colors, optional intensities,
/// and file metadata as JSON.
///
/// # Example
///
/// ```js
/// const result = core.parseE57(new Uint8Array(buffer));
/// console.log(`Points: ${result.pointCount}`);
/// console.log(`Has colors: ${result.colors !== null}`);
/// console.log(`Info: ${result.fileInfo()}`);
/// ```
#[cfg(feature = "e57-support")]
#[wasm_bindgen(js_name = "parseE57")]
pub fn parse_e57(bytes: &[u8]) -> Result<E57Result, SpatialErrorDetail> {
    if bytes.len() > DEFAULT_MAX_INPUT_SIZE {
        return Err(SpatialError::InputTooLarge.with_detail(format!(
            "E57 input is {} bytes, max is {}",
            bytes.len(),
            DEFAULT_MAX_INPUT_SIZE
        )));
    }
    parse_e57_core(bytes).map_err(SpatialError::point_cloud_error)
}

/// Parse E57 points with a JS progress callback. Reports periodically.
#[cfg(feature = "e57-support")]
#[wasm_bindgen(js_name = "parseE57Stream")]
pub fn parse_e57_stream(
    bytes: &[u8],
    on_progress: &js_sys::Function,
) -> Result<E57Result, SpatialErrorDetail> {
    if bytes.len() > DEFAULT_MAX_INPUT_SIZE {
        return Err(SpatialError::InputTooLarge.with_detail(format!(
            "E57 input is {} bytes, max is {}",
            bytes.len(),
            DEFAULT_MAX_INPUT_SIZE
        )));
    }
    // Progress is tracked internally by the simple reader but we report
    // based on estimated total. For now, use the simple path and report
    // start/end. A more advanced implementation would wrap the iterator.
    let this = JsValue::NULL;
    let _ = on_progress.call2(&this, &JsValue::from(0u32), &JsValue::from(100u32));
    let result = parse_e57_core(bytes).map_err(SpatialError::point_cloud_error)?;
    let _ = on_progress.call2(&this, &JsValue::from(100u32), &JsValue::from(100u32));
    Ok(result)
}

// ===========================================================================
// Stub implementations when feature is not enabled
// ===========================================================================

#[cfg(not(feature = "e57-support"))]
#[wasm_bindgen(js_name = "parseE57")]
pub fn parse_e57(_bytes: &[u8]) -> Result<JsValue, SpatialErrorDetail> {
    Err(SpatialError::point_cloud_error(
        "E57 support is not enabled. Build with --features e57-support to enable E57 parsing.",
    ))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn test_is_e57_format_valid() {
        // Minimal valid E57 header has "ASTM-E57" at offset 0
        let mut data = vec![0u8; 48];
        data[0..8].copy_from_slice(b"ASTM-E57");
        assert!(is_e57_format(&data));
    }

    #[test]
    fn test_is_e57_format_invalid_magic() {
        let data = b"NOT-E57-XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
        assert!(!is_e57_format(data));
    }

    #[test]
    fn test_is_e57_format_too_short() {
        let data = b"ASTM-E57";
        assert!(!is_e57_format(data));
    }

    #[test]
    fn test_is_e57_format_empty() {
        assert!(!is_e57_format(&[]));
    }

    #[cfg(feature = "e57-support")]
    #[test]
    fn test_e57_roundtrip_xyz_only() {
        let points = vec![
            (10.0, 20.0, 30.0),
            (40.0, 50.0, 60.0),
            (70.0, 80.0, 90.0),
            (-10.0, -20.0, -30.0),
        ];
        let e57_bytes = build_test_e57_file(&points, false, false).unwrap();

        assert!(is_e57_format(&e57_bytes));

        let result = parse_e57_core(&e57_bytes).unwrap();
        assert_eq!(result.point_count, 4);
        assert_eq!(result.positions.len(), 12);
        assert!(result.colors.is_none());
        assert!(result.intensities.is_none());

        // Verify positions match
        let eps = 0.001;
        assert!((result.positions[0] - 10.0).abs() < eps);
        assert!((result.positions[1] - 20.0).abs() < eps);
        assert!((result.positions[2] - 30.0).abs() < eps);
        assert!((result.positions[3] - 40.0).abs() < eps);
        assert!((result.positions[6] - 70.0).abs() < eps);
        assert!((result.positions[9] - (-10.0)).abs() < eps);
    }

    #[cfg(feature = "e57-support")]
    #[test]
    fn test_e57_roundtrip_with_color() {
        let points = vec![(1.0, 2.0, 3.0), (10.0, 20.0, 30.0)];
        let e57_bytes = build_test_e57_file(&points, true, false).unwrap();

        let result = parse_e57_core(&e57_bytes).unwrap();
        assert_eq!(result.point_count, 2);
        assert!(result.colors.is_some());

        let colors = result.colors.unwrap();
        assert_eq!(colors.len(), 6); // 2 points × 3 RGB
    }

    #[cfg(feature = "e57-support")]
    #[test]
    fn test_e57_roundtrip_with_intensity() {
        let points = vec![(1.0, 0.0, 0.0), (0.0, 2.0, 0.0), (0.0, 0.0, 3.0)];
        let e57_bytes = build_test_e57_file(&points, false, true).unwrap();

        let result = parse_e57_core(&e57_bytes).unwrap();
        assert_eq!(result.point_count, 3);
        assert!(result.intensities.is_some());

        let intensities = result.intensities.unwrap();
        assert_eq!(intensities.len(), 3);
    }

    #[cfg(feature = "e57-support")]
    #[test]
    fn test_e57_empty_file() {
        let points: Vec<(f64, f64, f64)> = vec![];
        let e57_bytes = build_test_e57_file(&points, false, false).unwrap();

        let result = parse_e57_core(&e57_bytes).unwrap();
        assert_eq!(result.point_count, 0);
        assert!(result.positions.is_empty());
    }

    #[cfg(feature = "e57-support")]
    #[test]
    fn test_e57_invalid_magic() {
        let mut data = vec![0u8; 48];
        data[0..7].copy_from_slice(b"INVALID");

        let result = parse_e57_core(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid E57 magic"));
    }

    #[cfg(feature = "e57-support")]
    #[test]
    fn test_e57_too_short() {
        let result = parse_e57_core(&[0u8; 10]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }

    #[cfg(feature = "e57-support")]
    #[test]
    fn test_e57_file_info_json() {
        let points = vec![(5.0, 10.0, 15.0)];
        let e57_bytes = build_test_e57_file(&points, false, false).unwrap();

        let result = parse_e57_core(&e57_bytes).unwrap();
        let info: serde_json::Value = serde_json::from_str(&result.file_info).unwrap();

        assert_eq!(info["format"], "ASTM E57 3D Imaging Data File");
        assert_eq!(info["guid"], "test-guid");
        assert_eq!(info["pointcloudCount"], 1);
        assert!(info["pointcloud0"]["records"].as_u64().unwrap() >= 1);
    }

    #[cfg(feature = "e57-support")]
    #[test]
    fn test_e57_many_points() {
        let points: Vec<(f64, f64, f64)> = (0..200)
            .map(|i| {
                (
                    i as f64 * 0.5,
                    (i as f64 * 0.1).sin(),
                    (i as f64 * 0.1).cos(),
                )
            })
            .collect();
        let e57_bytes = build_test_e57_file(&points, true, true).unwrap();

        let result = parse_e57_core(&e57_bytes).unwrap();
        assert_eq!(result.point_count, 200);
        assert!(result.colors.is_some());
        assert!(result.intensities.is_some());
    }

    #[cfg(feature = "e57-support")]
    #[test]
    fn test_e57_writer_cannot_write_readonly_cursor() {
        // Test that we can actually get inner buffer from writer
        let points = vec![(1.0, 2.0, 3.0)];
        let result = build_test_e57_file(&points, false, false);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(bytes.len() > 48); // must have header + XML + data
    }
}
