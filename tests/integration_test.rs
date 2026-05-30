//! Cross-module integration tests.
//!
//! These tests exercise the full processing pipeline using native Rust
//! (core functions, not WASM bindings) to verify that modules work together.

// ── Pipeline 1: GeoJSON Parsing + Validation ──────────────────────────────

#[test]
fn test_pipeline_geojson_parse_valid() {
    let geojson_str = r#"{
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]]
                },
                "properties": {"name": "A"}
            },
            {
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [5.0, 5.0]
                },
                "properties": {"name": "B"}
            },
            {
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [50.0, 50.0]
                },
                "properties": {"name": "C"}
            }
        ]
    }"#;

    let parsed: geojson::GeoJson = geojson_str.parse().unwrap();
    if let geojson::GeoJson::FeatureCollection(fc) = parsed {
        assert_eq!(fc.features.len(), 3);

        // Feature 1: Polygon
        if let Some(geojson::Geometry {
            value: geojson::Value::Polygon(rings),
            ..
        }) = &fc.features[0].geometry
        {
            assert_eq!(rings[0].len(), 5);
        }

        // Feature 2: Point
        if let Some(geojson::Geometry {
            value: geojson::Value::Point(pt),
            ..
        }) = &fc.features[1].geometry
        {
            assert_eq!(pt[0], 5.0);
            assert_eq!(pt[1], 5.0);
        }
    }
}

// ── Pipeline 2: Coordinate Transform Roundtrip ───────────────────────────

#[test]
fn test_pipeline_coordinate_roundtrip() {
    let (lng, lat) = (116.397428_f64, 39.90923_f64);

    // WGS84 → GCJ02 → WGS84 roundtrip
    let mut gcj_coords = vec![lng, lat];
    wasm_spatial_core::batch_wgs84_to_gcj02_in_place(&mut gcj_coords);
    assert_ne!(gcj_coords[0], lng, "GCJ02 should offset coordinates");
    wasm_spatial_core::batch_gcj02_to_wgs84_in_place(&mut gcj_coords);
    // GCJ02 roundtrip has inherent precision loss
    assert!((gcj_coords[0] - lng).abs() < 0.00001);
    assert!((gcj_coords[1] - lat).abs() < 0.00001);

    // WGS84 → Mercator → WGS84 roundtrip
    let mut merc_coords = vec![lng, lat];
    wasm_spatial_core::batch_wgs84_to_mercator_in_place(&mut merc_coords);
    assert!(merc_coords[0].abs() < 20_000_000.0, "Mercator x should be in range");
    assert!(merc_coords[1].abs() < 20_000_000.0, "Mercator y should be in range");
    wasm_spatial_core::batch_mercator_to_wgs84_in_place(&mut merc_coords);
    assert!((merc_coords[0] - lng).abs() < 1e-10);
    assert!((merc_coords[1] - lat).abs() < 1e-10);

    // WGS84 → BD09 → WGS84 roundtrip
    let mut bd_coords = vec![lng, lat];
    wasm_spatial_core::batch_wgs84_to_bd09_in_place(&mut bd_coords);
    assert_ne!(bd_coords[0], lng);
    wasm_spatial_core::batch_bd09_to_wgs84_in_place(&mut bd_coords);
    // BD09 roundtrip has inherent precision loss
    assert!((bd_coords[0] - lng).abs() < 0.00001);
    assert!((bd_coords[1] - lat).abs() < 0.00001);

    // GCJ02 → BD09 → GCJ02 roundtrip
    let mut gcj2_coords = vec![lng, lat];
    wasm_spatial_core::batch_wgs84_to_gcj02_in_place(&mut gcj2_coords);
    let gcj_lng = gcj2_coords[0];
    let gcj_lat = gcj2_coords[1];
    wasm_spatial_core::batch_gcj02_to_bd09_in_place(&mut gcj2_coords);
    wasm_spatial_core::batch_bd09_to_gcj02_in_place(&mut gcj2_coords);
    // BD09↔GCJ02 roundtrip has inherent precision loss
    assert!((gcj2_coords[0] - gcj_lng).abs() < 0.00001);
    assert!((gcj2_coords[1] - gcj_lat).abs() < 0.00001);
}

// ── Pipeline 3: Batch coordinate transform ─────────────────────────────

#[test]
fn test_pipeline_batch_coordinate_transform() {
    let mut coords: Vec<f64> = Vec::new();
    for i in 0..100 {
        coords.push(110.0 + i as f64 * 0.01);
        coords.push(30.0 + i as f64 * 0.01);
    }
    let original = coords.clone();

    wasm_spatial_core::batch_wgs84_to_gcj02_in_place(&mut coords);

    for i in 0..200 {
        assert_ne!(coords[i], original[i]);
    }

    wasm_spatial_core::batch_gcj02_to_wgs84_in_place(&mut coords);

    for i in 0..200 {
        assert!((coords[i] - original[i]).abs() < 0.001, "Roundtrip failed at i={}", i);
    }
}

// ── Pipeline 4: IFC Geometry Parsing ──────────────────────────────────────

#[test]
fn test_pipeline_ifc_geometry() {
    let ifc_text = r#"
#1=IFCEXTRUDEDAREASOLID(#2,#3,#4,5.0);
#2=IFCSHAPEREPRESENTATION('Box','Body',(#10),#100);
#3=IFCAXIS2PLACEMENT3D(#5,#6,$);
#4=IFCDIRECTION((0.0,0.0,1.0));
#5=IFCCARTESIANPOINT((0.0,0.0,0.0));
#6=IFCDIRECTION((0.0,1.0,0.0));
#10=IFCPOLYLINE((#11,#12,#13));
#11=IFCCARTESIANPOINT((0.0,0.0));
#12=IFCCARTESIANPOINT((3.0,0.0));
#13=IFCCARTESIANPOINT((1.5,2.0));
#100=IFCAXIS2PLACEMENT3D(#5,#6,$);
"#;

    let result = wasm_spatial_core::parse_ifc_geometry_core(ifc_text);
    assert_eq!(result.mesh_count(), 1);

    let mesh = &result.get_meshes()[0];
    let vc = (mesh.get_positions().len() / 3) as u32;
    assert!(vc > 0);
    assert!(!mesh.get_indices().is_empty());

    // Verify all indices are valid
    for &idx in mesh.get_indices() {
        assert!(idx < vc);
    }

    // Verify counts are consistent
    assert_eq!(mesh.get_positions().len() % 3, 0);
    assert_eq!(mesh.get_indices().len() % 3, 0);
}

// ── Pipeline 5: Spatial Analysis Cross-Checks ─────────────────────────────

#[test]
fn test_pipeline_spatial_analysis_consistency() {
    let coords: Vec<f64> = vec![
        0.0, 0.0,
        10.0, 0.0,
        10.0, 10.0,
        0.0, 10.0,
    ];
    let point_count = coords.len() / 2;

    let mut min_lng = f64::MAX;
    let mut min_lat = f64::MAX;
    let mut max_lng = f64::MIN;
    let mut max_lat = f64::MIN;
    for i in 0..point_count {
        min_lng = min_lng.min(coords[i * 2]);
        min_lat = min_lat.min(coords[i * 2 + 1]);
        max_lng = max_lng.max(coords[i * 2]);
        max_lat = max_lat.max(coords[i * 2 + 1]);
    }

    let sum_lng: f64 = (0..point_count).map(|i| coords[i * 2]).sum();
    let sum_lat: f64 = (0..point_count).map(|i| coords[i * 2 + 1]).sum();
    let centroid = (sum_lng / point_count as f64, sum_lat / point_count as f64);

    // Centroid should be inside bounding box
    assert!(centroid.0 >= min_lng && centroid.0 <= max_lng);
    assert!(centroid.1 >= min_lat && centroid.1 <= max_lat);
    assert_eq!(centroid.0, (min_lng + max_lng) / 2.0);
    assert_eq!(centroid.1, (min_lat + max_lat) / 2.0);
    assert_eq!((max_lng - min_lng) * (max_lat - min_lat), 100.0);
}

// ── Pipeline 6: Point Cloud Simulation → Decimation ───────────────────────

#[cfg(feature = "point-cloud")]
#[test]
fn test_pipeline_pointcloud_decimation() {
    let positions: Vec<f32> = (0..100)
        .flat_map(|i| {
            let x = (i % 10) as f32;
            let y = (i / 10) as f32;
            vec![x, y, 0.0f32]
        })
        .collect();
    let colors: Vec<u8> = (0..300).map(|i| (i % 256) as u8).collect();

    let (dec_pos, dec_col) = wasm_spatial_core::voxel_grid_decimate_core(&positions, &colors, 5.0);
    assert!(!dec_pos.is_empty());
    assert!(dec_pos.len() <= positions.len());

    let (rand_pos, rand_col) = wasm_spatial_core::random_decimate_core(&dec_pos, &dec_col, 10);
    // random_decimate returns min(target, available) points × 3
    let rand_point_count = rand_pos.len() / 3;
    assert!(rand_point_count > 0 && rand_point_count <= 10);
    assert_eq!(rand_col.len(), rand_point_count * 3);
}

// ── Pipeline 7: GeoJSON polygon validation for triangulation ──────────────

#[test]
fn test_pipeline_geojson_polygon_triangulation_precheck() {
    let geojson_str = r#"{
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]
        },
        "properties": {}
    }"#;

    let parsed: geojson::GeoJson = geojson_str.parse().unwrap();
    if let geojson::GeoJson::Feature(feature) = parsed {
        if let Some(geojson::Geometry {
            value: geojson::Value::Polygon(rings),
            ..
        }) = &feature.geometry
        {
            let ring = &rings[0];
            assert_eq!(ring[0], ring[ring.len() - 1]); // closed
            assert!(ring.len() >= 4); // triangle + close
        }
    }
}

// ── Pipeline 8: LAS Header + Range Access ───────────────────────────────

#[cfg(feature = "point-cloud")]
#[test]
fn test_pipeline_las_header_range_access() {
    use wasm_spatial_core::{read_u32_le, read_u16_le, parse_las_header_core};

    // Build a minimal LAS file with 3 points
    let points: Vec<(f64, f64, f64)> = vec![(10.0, 20.0, 30.0), (40.0, 50.0, 60.0), (70.0, 80.0, 90.0)];
    let num_points = points.len() as u32;
    let header_size = 230u32;
    let point_format: u8 = 0;
    let record_len: u16 = 20;

    let mut buf = vec![0u8; header_size as usize];
    buf[0..4].copy_from_slice(b"LASF");
    buf[96..98].copy_from_slice(&(header_size as u16).to_le_bytes());
    buf[98..102].copy_from_slice(&header_size.to_le_bytes()); // point offset = header_size
    buf[106] = point_format;
    buf[108..110].copy_from_slice(&record_len.to_le_bytes());
    buf[110..114].copy_from_slice(&num_points.to_le_bytes());
    buf[134..142].copy_from_slice(&1.0f64.to_le_bytes());
    buf[142..150].copy_from_slice(&1.0f64.to_le_bytes());
    buf[150..158].copy_from_slice(&1.0f64.to_le_bytes());
    buf[182..190].copy_from_slice(&90.0f64.to_le_bytes());
    buf[190..198].copy_from_slice(&90.0f64.to_le_bytes());
    buf[198..206].copy_from_slice(&90.0f64.to_le_bytes());
    buf[206..214].copy_from_slice(&10.0f64.to_le_bytes());
    buf[214..222].copy_from_slice(&20.0f64.to_le_bytes());
    buf[222..230].copy_from_slice(&30.0f64.to_le_bytes());

    for &(x, y, z) in &points {
        let base = buf.len();
        buf.resize(base + record_len as usize, 0);
        buf[base..base + 4].copy_from_slice(&(x as i32).to_le_bytes());
        buf[base + 4..base + 8].copy_from_slice(&(y as i32).to_le_bytes());
        buf[base + 8..base + 12].copy_from_slice(&(z as i32).to_le_bytes());
    }

    let header = parse_las_header_core(&buf).unwrap();
    assert_eq!(header.num_points(), 3);
    assert_eq!(header.point_format_id(), 0);
    assert_eq!(header.point_data_record_length(), 20);

    let p_offset = read_u32_le(&buf, 98);
    let p_record_len = read_u16_le(&buf, 108);
    for i in 0..3u32 {
        let expected = p_offset as usize + i as usize * p_record_len as usize;
        assert_eq!(expected, 230 + i as usize * 20);
    }
}
