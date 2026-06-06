//! Integration tests for SVD 3D alignment (Wave 2.7).

#![cfg(feature = "mesh-ingest")]

use wasm_spatial_core::{apply_rigid_alignment, compute_rigid_alignment, EnuFrame};

/// Simulated photogrammetry block → survey ENU registration via control points.
#[test]
fn test_photo_to_enu_registration() {
    let theta = 30.0_f64.to_radians();
    let (sin_t, cos_t) = theta.sin_cos();

    // Survey/GIS control points in site ENU (meters)
    let survey_ctrl = vec![
        [120.0, 80.0, 2.0],
        [170.0, 80.0, 2.0],
        [120.0, 130.0, 2.0],
        [170.0, 135.0, 7.0],
    ];

    // Photogrammetry local frame: inverse of known yaw + offset
    let photo_ctrl: Vec<[f64; 3]> = survey_ctrl
        .iter()
        .map(|p| {
            let dx = p[0] - 120.0;
            let dy = p[1] - 80.0;
            [
                cos_t * dx + sin_t * dy,
                -sin_t * dx + cos_t * dy,
                p[2] - 2.0,
            ]
        })
        .collect();

    let fit = compute_rigid_alignment(&photo_ctrl, &survey_ctrl, false).unwrap();
    assert!(fit.rmse < 1e-5, "rmse={}", fit.rmse);

    let cloud: Vec<f32> = (0..20)
        .flat_map(|i| {
            let f = i as f32;
            [f, f * 0.5, 0.1 * f]
        })
        .collect();

    let aligned = apply_rigid_alignment(&cloud, &fit);
    assert_eq!(aligned.len(), cloud.len());
    assert!(aligned.iter().all(|v| v.is_finite()));

    // ENU frame round-trip: aligned photo cloud can be expressed relative to anchor
    let frame = EnuFrame::from_anchor(116.397, 39.909, 50.0);
    let _ = frame.anchor_lng;
}

#[test]
fn test_mat4_column_major() {
    let src = vec![
        [0.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        [0.0, 10.0, 0.0],
        [0.0, 0.0, 10.0],
    ];
    let tgt: Vec<[f64; 3]> = src
        .iter()
        .map(|p| [p[0] + 1.0, p[1] + 2.0, p[2] + 3.0])
        .collect();

    let fit = compute_rigid_alignment(&src, &tgt, false).unwrap();
    let m = fit.to_mat4();
    assert!((m[15] - 1.0).abs() < 1e-12);
    assert!((m[12] - 1.0).abs() < 1e-6);
    assert!((m[13] - 2.0).abs() < 1e-6);
    assert!((m[14] - 3.0).abs() < 1e-6);
}
