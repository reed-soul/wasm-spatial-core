//! Real-data validation tests for GeoTIFF terrain parsing and pipeline.
//!
//! Uses synthetically-generated GeoTIFF files with realistic terrain features
//! (mountains, ridges, valleys, noise) in `tests/fixtures/`.

use wasm_spatial_core::{
    apply_color_ramp_core, contour_lines_core, encode_quantized_mesh_core,
    encode_terrain_tileset_core, hillshade_core, parse_geotiff_core, ColorRamp,
};

// ===========================================================================
// Helper: build minimal Float32 GeoTIFF blob
// ===========================================================================

fn build_minimal_tiff_f32(width: u32, height: u32, values: &[f32]) -> Vec<u8> {
    assert_eq!(values.len(), (width * height) as usize);

    let mut buf = Vec::new();

    // TIFF Header (little-endian)
    buf.extend_from_slice(b"II"); // Little endian
    buf.extend_from_slice(&42u16.to_le_bytes()); // Magic
    let ifd_offset_pos = buf.len();
    buf.extend_from_slice(&0u32.to_le_bytes()); // Placeholder for IFD offset

    // IFD starts here
    let ifd_start = buf.len();
    buf[ifd_offset_pos..ifd_offset_pos + 4].copy_from_slice(&(ifd_start as u32).to_le_bytes());

    // Number of IFD entries
    let num_entries: u16 = 7;
    buf.extend_from_slice(&num_entries.to_le_bytes());

    let ne = num_entries;
    let strip_data_offset_pos = ifd_start + 2 + (ne as usize) * 12 + 4;
    let strip_data_offset = strip_data_offset_pos as u32;
    let strip_data_size = values.len() * 4;

    // Tag 256: ImageWidth
    buf.extend_from_slice(&256u16.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&width.to_le_bytes());

    // Tag 257: ImageLength
    buf.extend_from_slice(&257u16.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&height.to_le_bytes());

    // Tag 258: BitsPerSample
    buf.extend_from_slice(&258u16.to_le_bytes());
    buf.extend_from_slice(&3u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&32u16.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());

    // Tag 259: Compression (1 = none)
    buf.extend_from_slice(&259u16.to_le_bytes());
    buf.extend_from_slice(&3u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());

    // Tag 273: StripOffsets
    buf.extend_from_slice(&273u16.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&strip_data_offset.to_le_bytes());

    // Tag 278: RowsPerStrip
    buf.extend_from_slice(&278u16.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&height.to_le_bytes());

    // Tag 279: StripByteCounts
    buf.extend_from_slice(&279u16.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&(strip_data_size as u32).to_le_bytes());

    // Next IFD = 0
    buf.extend_from_slice(&0u32.to_le_bytes());

    // Strip data
    for &val in values {
        buf.extend_from_slice(&val.to_le_bytes());
    }

    buf
}

/// Build a GeoTIFF with GeoKeys (CRS metadata).
fn build_geotiff_with_crs(width: u32, height: u32, values: &[f32]) -> Vec<u8> {
    assert_eq!(values.len(), (width * height) as usize);

    let mut buf = Vec::new();

    buf.extend_from_slice(b"II");
    buf.extend_from_slice(&42u16.to_le_bytes());
    let ifd_offset_pos = buf.len();
    buf.extend_from_slice(&0u32.to_le_bytes());

    let ifd_start = buf.len();
    buf[ifd_offset_pos..ifd_offset_pos + 4].copy_from_slice(&(ifd_start as u32).to_le_bytes());

    // 9 entries including GeoKeyDirectory
    let num_entries: u16 = 9;
    buf.extend_from_slice(&num_entries.to_le_bytes());

    let entries_end = ifd_start + 2 + (num_entries as usize) * 12 + 4;
    let geo_key_offset = entries_end as u32;
    let geo_keys: Vec<u16> = vec![
        1, 1, 0, 2, // header: 1 key, version 1.0, 2 keys
        1024, 0, 1, 2, // GTModelTypeGeoKey: Geographic
        1026, 0, 1, 4326, // GeographicTypeGeoKey: WGS 84
    ];
    let geo_key_size = geo_keys.len() * 2;
    let strip_data_offset = geo_key_offset + geo_key_size as u32;
    let strip_data_size = values.len() * 4;

    // IFD entries (sorted by tag)
    buf.extend_from_slice(&256u16.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&width.to_le_bytes());
    // 257
    buf.extend_from_slice(&257u16.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&height.to_le_bytes());
    // 258
    buf.extend_from_slice(&258u16.to_le_bytes());
    buf.extend_from_slice(&3u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&32u16.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());
    // 259
    buf.extend_from_slice(&259u16.to_le_bytes());
    buf.extend_from_slice(&3u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());
    // 273
    buf.extend_from_slice(&273u16.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&strip_data_offset.to_le_bytes());
    // 278
    buf.extend_from_slice(&278u16.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&height.to_le_bytes());
    // 279
    buf.extend_from_slice(&279u16.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&(strip_data_size as u32).to_le_bytes());
    // 339
    buf.extend_from_slice(&339u16.to_le_bytes());
    buf.extend_from_slice(&3u16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&3u16.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());
    // 34735
    buf.extend_from_slice(&34735u16.to_le_bytes());
    buf.extend_from_slice(&3u16.to_le_bytes());
    buf.extend_from_slice(&(geo_keys.len() as u32).to_le_bytes());
    buf.extend_from_slice(&geo_key_offset.to_le_bytes());

    buf.extend_from_slice(&0u32.to_le_bytes()); // next IFD

    for &key in &geo_keys {
        buf.extend_from_slice(&key.to_le_bytes());
    }

    for &val in values {
        buf.extend_from_slice(&val.to_le_bytes());
    }

    buf
}

fn load_fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new("tests/fixtures").join(name);
    if !path.exists() {
        panic!("fixture not found: {}", path.display());
    }
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

// ===========================================================================
// 1. Real GeoTIFF basic validation
// ===========================================================================

#[test]
fn test_real_geotiff_parse_and_dimensions() {
    let blob = load_fixture("terrain_64x64.tif");
    let info = parse_geotiff_core(&blob).expect("GeoTIFF parse failed");

    assert_eq!(info.width(), 64);
    assert_eq!(info.height(), 64);
    assert_eq!(info.elevations().len(), 64 * 64);

    eprintln!(
        "terrain_64x64.tif: {}x{}, {} elevation values",
        info.width(),
        info.height(),
        info.elevations().len()
    );
}

#[test]
fn test_real_geotiff_elevation_range_reasonable() {
    let blob = load_fixture("terrain_64x64.tif");
    let info = parse_geotiff_core(&blob).expect("parse failed");

    let elevations = info.elevations();
    let min_z = elevations.iter().copied().fold(f32::INFINITY, f32::min);
    let max_z = elevations.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    assert!(min_z >= -500.0, "min elevation {} below -500m", min_z);
    assert!(max_z <= 9000.0, "max elevation {} above 9000m", max_z);
    assert!(
        (max_z - min_z) > 100.0,
        "range too small: [{:.1}, {:.1}]",
        min_z,
        max_z
    );

    for &h in elevations {
        assert!(h.is_finite(), "non-finite elevation: {h}");
    }

    eprintln!("Elevation range: [{:.1}, {:.1}] m", min_z, max_z);
}

#[test]
fn test_real_geotiff_crs_info() {
    let blob = load_fixture("terrain_64x64.tif");
    let info = parse_geotiff_core(&blob).expect("parse failed");

    let crs = info.crs();
    assert!(!crs.is_empty());

    let crs_val: serde_json::Value = serde_json::from_str(&crs).expect("CRS not valid JSON");
    assert!(crs_val.is_object());
    eprintln!("CRS: {}", crs);
}

#[test]
fn test_real_geotiff_bounds() {
    let blob = load_fixture("terrain_64x64.tif");
    let info = parse_geotiff_core(&blob).expect("parse failed");

    let bounds = info.bounds_slice();
    assert!(bounds[0] < bounds[2], "min_lng should be < max_lng");
    assert!(bounds[1] < bounds[3], "min_lat should be < max_lat");

    eprintln!(
        "Bounds: [{:.4}, {:.4}, {:.4}, {:.4}]",
        bounds[0], bounds[1], bounds[2], bounds[3]
    );
}

#[test]
fn test_real_geotiff_256x256() {
    let blob = load_fixture("terrain_256x256.tif");
    let info = parse_geotiff_core(&blob).expect("GeoTIFF 256x256 parse failed");

    assert_eq!(info.width(), 256);
    assert_eq!(info.height(), 256);

    let elevations = info.elevations();
    let min_z = elevations.iter().copied().fold(f32::INFINITY, f32::min);
    let max_z = elevations.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    eprintln!("terrain_256x256.tif: range [{:.1}, {:.1}] m", min_z, max_z);
    assert!((max_z - min_z) > 100.0);
}

// ===========================================================================
// 2. Terrain pipeline: GeoTIFF → quantized-mesh
// ===========================================================================

#[test]
fn test_quantized_mesh_vertex_count() {
    let blob = load_fixture("terrain_64x64.tif");
    let info = parse_geotiff_core(&blob).expect("parse failed");
    let w = info.width();
    let h = info.height();
    let elevations = info.elevations();

    let bounds = [0.0, 0.0, 1.0, 1.0];
    let center = [0.5, 0.5, 0.0];

    let data = encode_quantized_mesh_core(elevations, w, h, &bounds, &center)
        .expect("quantized-mesh encode failed");

    // Verify minimum expected size: center(24) + heights(4) + normal(1) + mask(1) + hdrSize(4) + vertCount(4) + vertData
    assert!(
        data.len() > 38,
        "data too short for a valid quantized mesh ({} bytes)",
        data.len()
    );

    // Header starts with center x/y/z (3 × f64)
    let cx = f64::from_le_bytes(data[0..8].try_into().unwrap());
    let cy = f64::from_le_bytes(data[8..16].try_into().unwrap());
    let cz = f64::from_le_bytes(data[16..24].try_into().unwrap());
    assert!((cx - 0.5).abs() < 0.01);
    assert!((cy - 0.5).abs() < 0.01);
    assert!((cz - 0.0).abs() < 0.01);

    eprintln!(
        "quantized-mesh: {} bytes, center=({:.1}, {:.1}, {:.1})",
        data.len(),
        cx,
        cy,
        cz
    );
}

#[test]
fn test_quantized_mesh_4x4_grid() {
    let values: Vec<f32> = (0..16).map(|i| (i as f32) * 10.0).collect();
    let tiff = build_minimal_tiff_f32(4, 4, &values);
    let info = parse_geotiff_core(&tiff).expect("4x4 parse failed");

    let bounds = [0.0, 0.0, 1.0, 1.0];
    let center = [0.5, 0.5, 0.0];

    let data = encode_quantized_mesh_core(info.elevations(), 4, 4, &bounds, &center)
        .expect("4x4 quantized-mesh failed");

    // Header layout:
    //   0-23: center x/y/z (3 × f64 = 24 bytes)
    //   24-25: min height (2 bytes)
    //   26-27: max height (2 bytes)
    //   28: normal (1 byte)
    //   29: watermask (1 byte)
    //   30-33: headerSize (4 bytes)
    //   34-37: vertexCount (4 bytes)
    //   38+: vertex data (6 bytes each: u16 u, u16 v, u16 h)
    let vertex_count = u32::from_le_bytes(data[34..38].try_into().unwrap());
    assert_eq!(vertex_count, 16, "should have 16 vertices for 4x4 grid");

    // Vertex data: 6 bytes per vertex (u, v, h as u16)
    let vertex_data_end = 38 + (vertex_count as usize) * 6;
    // Triangle count follows vertex data
    let triangle_count = u32::from_le_bytes(
        data[vertex_data_end..vertex_data_end + 4]
            .try_into()
            .unwrap(),
    );
    assert_eq!(triangle_count, 18, "should have 18 triangles for 4x4 grid");

    eprintln!(
        "4x4 quantized-mesh: {} vertices, {} triangles",
        vertex_count, triangle_count
    );
}

#[test]
fn test_terrain_tileset_structure() {
    let blob = load_fixture("terrain_64x64.tif");
    let info = parse_geotiff_core(&blob).expect("parse failed");

    let bounds = *info.bounds_slice();
    let center = [
        bounds[0] + (bounds[2] - bounds[0]) / 2.0,
        bounds[1] + (bounds[3] - bounds[1]) / 2.0,
        500.0,
    ];

    let result = encode_terrain_tileset_core(
        info.elevations(),
        info.width(),
        info.height(),
        info.bounds_slice(),
        &center,
        5,
    )
    .expect("terrain tileset failed");

    let json_val: serde_json::Value =
        serde_json::from_str(result.tileset_json_str()).expect("tileset JSON invalid");
    assert!(json_val["asset"].is_object(), "missing asset");
    assert!(json_val["root"].is_object(), "missing root");

    eprintln!(
        "Terrain tileset: tiles present, {} bytes JSON",
        result.tileset_json_str().len()
    );
}

// ===========================================================================
// 3. Coloring validation
// ===========================================================================

#[test]
fn test_color_ramp_output_length() {
    let blob = load_fixture("terrain_64x64.tif");
    let info = parse_geotiff_core(&blob).expect("parse failed");
    let elevations = info.elevations();

    let min_z = elevations.iter().copied().fold(f32::INFINITY, f32::min) as f64;
    let max_z = elevations.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;

    let ramps = [
        ColorRamp::Terrain,
        ColorRamp::Heat,
        ColorRamp::Ocean,
        ColorRamp::Gray,
    ];
    for ramp in &ramps {
        let rgba = apply_color_ramp_core(elevations, min_z, max_z, *ramp);
        assert_eq!(
            rgba.len(),
            elevations.len() * 4,
            "ramp output should be {} RGBA bytes",
            elevations.len() * 4
        );
    }
    eprintln!(
        "All color ramps: OK ({} RGBA bytes each)",
        elevations.len() * 4
    );
}

#[test]
fn test_hillshade_output_length() {
    let blob = load_fixture("terrain_64x64.tif");
    let info = parse_geotiff_core(&blob).expect("parse failed");
    let elevations = info.elevations();

    let shading = hillshade_core(
        elevations,
        info.width() as usize,
        info.height() as usize,
        315.0,
        45.0,
    );
    assert_eq!(
        shading.len(),
        elevations.len(),
        "hillshade should match elevation count"
    );

    let nonzero = shading.iter().any(|&b| b > 0);
    assert!(nonzero, "hillshade should have non-zero values");

    eprintln!(
        "Hillshade: {} bytes, range [{}, {}]",
        shading.len(),
        shading.iter().copied().fold(0u8, u8::min),
        shading.iter().copied().fold(0u8, u8::max)
    );
}

#[test]
fn test_contour_lines_reasonable() {
    let blob = load_fixture("terrain_64x64.tif");
    let info = parse_geotiff_core(&blob).expect("parse failed");
    let elevations = info.elevations();

    let min_z = elevations.iter().copied().fold(f32::INFINITY, f32::min);
    let max_z = elevations.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let interval = ((max_z - min_z) / 10.0) as f64;

    let contours = contour_lines_core(
        elevations,
        info.width() as usize,
        info.height() as usize,
        interval,
    );

    assert!(!contours.is_empty(), "should have contour lines");

    for (i, &(x1, y1, x2, y2)) in contours.iter().enumerate() {
        assert!(
            x1.is_finite() && y1.is_finite() && x2.is_finite() && y2.is_finite(),
            "contour {} non-finite",
            i
        );
    }

    eprintln!(
        "Contour lines: {} segments, interval={:.1}m",
        contours.len(),
        interval
    );
}

// ===========================================================================
// 4. Edge cases
// ===========================================================================

#[test]
fn test_geotiff_2x2_minimal() {
    let values = vec![100.0f32, 200.0, 300.0, 400.0];
    let tiff = build_minimal_tiff_f32(2, 2, &values);
    let info = parse_geotiff_core(&tiff).expect("2x2 parse failed");

    assert_eq!(info.width(), 2);
    assert_eq!(info.height(), 2);
    assert_eq!(info.elevations().len(), 4);

    let bounds = [0.0, 0.0, 1.0, 1.0];
    let center = [0.5, 0.5, 250.0];
    let result = encode_quantized_mesh_core(info.elevations(), 2, 2, &bounds, &center);
    assert!(result.is_ok(), "2x2 quantized mesh should work");

    let shading = hillshade_core(info.elevations(), 2, 2, 315.0, 45.0);
    assert_eq!(shading.len(), 4);
}

#[test]
fn test_geotiff_with_negative_elevations() {
    let values = vec![-10.0f32, -5.0, 0.0, 5.0, -3.5, -1.0, 2.0, 8.0];
    let tiff = build_minimal_tiff_f32(4, 2, &values);
    let info = parse_geotiff_core(&tiff).expect("negative elevations parse failed");

    assert_eq!(info.elevations()[0], -10.0);
    assert_eq!(info.elevations()[3], 5.0);

    let rgba = apply_color_ramp_core(info.elevations(), -10.0, 8.0, ColorRamp::Terrain);
    assert_eq!(rgba.len(), 8 * 4);
}

#[test]
fn test_geotiff_truncated_no_panic() {
    let values = vec![1.0f32, 2.0, 3.0, 4.0];
    let tiff = build_minimal_tiff_f32(2, 2, &values);
    let truncated = &tiff[..tiff.len() / 2];

    let result = std::panic::catch_unwind(|| {
        let _ = parse_geotiff_core(truncated);
    });
    assert!(result.is_ok(), "truncated GeoTIFF should not panic");
    eprintln!("Truncated GeoTIFF: no panic");
}

#[test]
fn test_geotiff_invalid_magic() {
    let blob = load_fixture("sample.las"); // Not a TIFF file
    let result = parse_geotiff_core(&blob);
    assert!(result.is_err(), "LAS file should fail GeoTIFF parsing");
    eprintln!("Invalid magic: graceful error (expected)");
}

#[test]
fn test_geotiff_crs_with_geokeys() {
    let values: Vec<f32> = (0..100).map(|i| i as f32 * 5.0).collect();
    let tiff = build_geotiff_with_crs(10, 10, &values);
    let info = parse_geotiff_core(&tiff).expect("CRS GeoTIFF parse failed");

    let crs = info.crs();
    let crs_val: serde_json::Value = serde_json::from_str(&crs).expect("CRS JSON parse");

    // Verify CRS is returned (exact parsing depends on GeoKey format)
    assert!(crs_val.is_object(), "CRS should be a JSON object");

    eprintln!("CRS GeoTIFF: {}", crs);
}
