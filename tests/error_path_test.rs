//! Error path tests for all WASM-exported functions.
//!
//! Ensures every function handles edge cases gracefully without panics.

#[cfg(feature = "point-cloud")]
use wasm_spatial_core::{
    encode_pnts_tile, parse_las_header_core, parse_las_points_core, parse_pnts_header, Octree,
};

// ===========================================================================
// Octree error paths
// ===========================================================================

#[test]
#[cfg(feature = "point-cloud")]
fn test_octree_empty_positions() {
    let mut positions: Vec<f32> = vec![];
    let tree = Octree::build(&mut positions, 1000, 10);
    assert_eq!(tree.node_count(), 1);
    assert_eq!(tree.total_points(), 0);
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_octree_single_point() {
    let mut positions = vec![42.0_f32, -17.5, 100.0];
    let tree = Octree::build(&mut positions, 1000, 10);
    assert_eq!(tree.node_count(), 1);
    assert_eq!(tree.total_points(), 1);
    assert!(tree.nodes[0].is_leaf());
    assert_eq!(tree.nodes[0].point_count, 1);
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_octree_all_nan() {
    let mut positions: Vec<f32> = vec![
        f32::NAN,
        f32::NAN,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    let tree = Octree::build(&mut positions, 1000, 10);
    assert_eq!(tree.node_count(), 1);
    assert_eq!(tree.total_points(), 0);
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_octree_all_infinity() {
    let mut positions: Vec<f32> = vec![
        f32::INFINITY,
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    let tree = Octree::build(&mut positions, 1000, 10);
    assert_eq!(tree.node_count(), 1);
    assert_eq!(tree.total_points(), 0);
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_octree_mixed_nan_and_valid() {
    let mut positions: Vec<f32> = vec![
        1.0,
        2.0,
        3.0,
        f32::NAN,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    let tree = Octree::build(&mut positions, 1000, 10);
    assert_eq!(tree.total_points(), 2);
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_octree_non_multiple_of_3() {
    let mut positions = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let tree = Octree::build(&mut positions, 1000, 10);
    assert_eq!(tree.total_points(), 2);
    assert_eq!(positions.len(), 6);
}

// ===========================================================================
// PNTS error paths
// ===========================================================================

#[test]
#[cfg(feature = "point-cloud")]
fn test_pnts_empty_positions() {
    let result = encode_pnts_tile(&[], [0.0; 3], None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("0 points"));
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_pnts_single_point() {
    let positions = vec![1.0_f32, 2.0, 3.0];
    let tile = encode_pnts_tile(&positions, [1.0, 2.0, 3.0], None).unwrap();
    assert_eq!(&tile[0..4], b"pnts");
    let (header, _) = parse_pnts_header(&tile).unwrap();
    assert_eq!(header.version, 1);
    assert_eq!(header.feature_table_binary_byte_length, 12);
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_pnts_single_point_with_color() {
    let positions = vec![42.0_f32, -17.5, 100.0];
    let colors = vec![255u8, 128, 64];
    let tile = encode_pnts_tile(&positions, [0.0; 3], Some(&colors)).unwrap();
    let (header, _) = parse_pnts_header(&tile).unwrap();
    assert_eq!(header.feature_table_binary_byte_length, 15);
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_pnts_color_length_mismatch() {
    let positions = vec![1.0_f32, 2.0, 3.0];
    let colors = vec![255u8, 0];
    let result = encode_pnts_tile(&positions, [0.0; 3], Some(&colors));
    assert!(result.is_err());
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_pnts_parse_invalid_magic() {
    let data = vec![0u8; 28];
    let result = parse_pnts_header(&data);
    assert!(result.is_err());
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_pnts_parse_too_short() {
    let data = vec![0u8; 10];
    let result = parse_pnts_header(&data);
    assert!(result.is_err());
}

// ===========================================================================
// LAS parsing error paths
// ===========================================================================

#[test]
#[cfg(feature = "point-cloud")]
fn test_las_zero_bytes() {
    let result = parse_las_header_core(&[]);
    assert!(result.is_err());
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_las_too_short_for_header() {
    let result = parse_las_header_core(&[0u8; 100]);
    assert!(result.is_err());
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_las_invalid_magic() {
    let mut blob = vec![0u8; 230];
    blob[0..4].copy_from_slice(b"XASX");
    match parse_las_header_core(&blob) {
        Ok(_) => panic!("Should have errored on invalid magic"),
        Err(err) => assert!(err.contains("Invalid LAS magic"), "Got: {}", err),
    }
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_las_truncated_no_point_data() {
    // Build a valid internal-format LAS blob (using same offsets as our parser)
    let mut blob = vec![0u8; 230];
    blob[0..4].copy_from_slice(b"LASF");
    blob[24] = 1; // version major
    blob[26] = 2; // version minor
                  // num_points = 100
    blob[110..114].copy_from_slice(&100u32.to_le_bytes());
    // point_offset = 230
    blob[98..102].copy_from_slice(&230u32.to_le_bytes());
    // record length = 20
    blob[108..110].copy_from_slice(&20u16.to_le_bytes());
    blob[134..142].copy_from_slice(&1.0_f64.to_le_bytes()); // x_scale
    blob[142..150].copy_from_slice(&1.0_f64.to_le_bytes()); // y_scale
    blob[150..158].copy_from_slice(&1.0_f64.to_le_bytes()); // z_scale

    let cloud = parse_las_points_core(&blob).unwrap();
    assert_eq!(cloud.point_count(), 0);
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_las_header_partial_data() {
    let mut blob = vec![0u8; 230];
    blob[0..4].copy_from_slice(b"LASF");
    blob[24] = 1;
    blob[26] = 2;
    blob[98..102].copy_from_slice(&230u32.to_le_bytes());
    blob[106] = 0; // format 0
    blob[108..110].copy_from_slice(&20u16.to_le_bytes());
    blob[110..114].copy_from_slice(&5u32.to_le_bytes());
    blob[134..142].copy_from_slice(&1.0_f64.to_le_bytes());
    blob[142..150].copy_from_slice(&1.0_f64.to_le_bytes());
    blob[150..158].copy_from_slice(&1.0_f64.to_le_bytes());

    // Add 2 complete point records (40 bytes)
    blob.resize(230 + 40, 0);
    blob[230..234].copy_from_slice(&10_i32.to_le_bytes());
    blob[234..238].copy_from_slice(&20_i32.to_le_bytes());
    blob[238..242].copy_from_slice(&30_i32.to_le_bytes());
    blob[242..246].copy_from_slice(&40_i32.to_le_bytes());
    blob[246..250].copy_from_slice(&50_i32.to_le_bytes());
    blob[250..254].copy_from_slice(&60_i32.to_le_bytes());

    let cloud = parse_las_points_core(&blob).unwrap();
    assert_eq!(cloud.point_count(), 2);
}

#[test]
#[cfg(feature = "point-cloud")]
fn test_las_parse_points_too_short_buffer() {
    let result = parse_las_points_core(&[0u8; 100]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("too short"));
}
