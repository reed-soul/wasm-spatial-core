//! 100M-point performance validation.
//!
//! Run: cargo test --test 100m_perf --features "point-cloud test-helpers" --release -- --nocapture --test-threads=1
//!
//! Memory budget: ~3.5GB (LAS blob ~1.4GB + positions ~1.2GB + octree/tileset)

#![cfg(feature = "test-helpers")]

use rand::Rng;
use std::io::Write;
use std::time::Instant;
use wasm_spatial_core::test_exports::test_helpers::{get_point_count, get_positions};
use wasm_spatial_core::{generate_tileset, Octree};

/// Build a valid LAS 1.2 blob (Point10, 14 bytes/point) with n points in [0,1000]^3.
fn build_las(n: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let record_len: usize = 20; // LAS format 0: XYZ(12) + intensity(2) + return(1) + class(1) + scanAngle(1) + userData(1) + pointSourceID(2)
    let header_size: usize = 227;

    let mut points = vec![0u8; n * record_len];
    for i in 0..n {
        let o = i * record_len;
        let x = rng.gen_range(0..1000i32);
        let y = rng.gen_range(0..1000i32);
        let z = rng.gen_range(0..1000i32);
        points[o..o + 4].copy_from_slice(&x.to_le_bytes());
        points[o + 4..o + 8].copy_from_slice(&y.to_le_bytes());
        points[o + 8..o + 12].copy_from_slice(&z.to_le_bytes());
    }

    let mut hdr = vec![0u8; header_size];
    hdr[0..4].copy_from_slice(b"LASF");
    hdr[24] = 1;
    hdr[25] = 2; // version 1.2
    hdr[94..96].copy_from_slice(&227u16.to_le_bytes()); // header size
    hdr[96..100].copy_from_slice(&227u32.to_le_bytes()); // point offset
    hdr[100..104].copy_from_slice(&(n as u32).to_le_bytes()); // num points
    hdr[104] = 0; // point format 0
    hdr[105..107].copy_from_slice(&20u16.to_le_bytes()); // record length
    hdr[131..139].copy_from_slice(&1.0f64.to_le_bytes()); // x scale
    hdr[139..147].copy_from_slice(&1.0f64.to_le_bytes()); // y scale
    hdr[147..155].copy_from_slice(&1.0f64.to_le_bytes()); // z scale

    let mut blob = hdr;
    blob.extend_from_slice(&points);
    blob
}

#[test]
#[ignore] // Too slow for CI (~9s release, needs ~3.5GB RAM). Run: cargo test --test 100m_perf --features "point-cloud test-helpers" --release -- --ignored --nocapture --test-threads=1
fn test_100m_las_full_pipeline() {
    let n: usize = 100_000_000;
    println!("\n=== 100M LAS Full Pipeline ===");
    let total = Instant::now();

    // [1/4] Generate LAS blob
    print!(
        "[1/4] Generating {}M LAS ({:.1} GB)...",
        n / 1_000_000,
        n as f64 * 14.0 / 1e9
    );
    std::io::stdout().flush().unwrap();
    let t = Instant::now();
    let las_blob = build_las(n);
    println!(" {:.1}s", t.elapsed().as_secs_f64());

    // [2/4] Parse
    print!("[2/4] Parsing {}M points...", n / 1_000_000);
    std::io::stdout().flush().unwrap();
    let t = Instant::now();
    let cloud = wasm_spatial_core::parse_las_points_core(&las_blob).expect("parse failed");
    let count = get_point_count(&cloud);
    let mut positions = get_positions(&cloud).to_vec();
    drop(cloud);
    drop(las_blob);
    println!(" {} pts in {:.1}s", count, t.elapsed().as_secs_f64());
    assert_eq!(count, n as u32);

    // [3/4] Octree
    print!("[3/4] Building octree...");
    std::io::stdout().flush().unwrap();
    let t = Instant::now();
    let octree = Octree::build(&mut positions, 50000, 21);
    println!(
        " {} nodes, {} leaves in {:.1}s",
        octree.node_count(),
        octree.leaf_count(),
        t.elapsed().as_secs_f64()
    );
    assert!(
        octree.leaf_count() > 1,
        "expected multiple leaves, got {}",
        octree.leaf_count()
    );

    // [4/4] Tileset
    print!("[4/4] Generating tileset...");
    std::io::stdout().flush().unwrap();
    let t = Instant::now();
    let tileset = generate_tileset(&octree, &positions, None).expect("tileset failed");
    println!(
        " {} tiles, {:.1} MB in {:.1}s",
        tileset.tile_count(),
        tileset.total_bytes() as f64 / 1e6,
        t.elapsed().as_secs_f64()
    );
    assert!(tileset.tile_count() > 0);

    println!("[Total] {:.1}s", total.elapsed().as_secs_f64());
    println!("=== PASS ===");
}
