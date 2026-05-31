//! Performance stress tests for the point cloud processing pipeline.
//!
//! These tests print timing information for manual evaluation.
//! Run with: `cargo test --features point-cloud -- perf_test --nocapture`

use std::time::Instant;

#[cfg(feature = "point-cloud")]
use wasm_spatial_core::{generate_tileset, parse_pnts_header, Octree};

/// Generate synthetic point cloud data with scan-line pattern.
fn generate_points(count: usize, spread: f32) -> Vec<f32> {
    let mut positions = Vec::with_capacity(count * 3);
    let rows = (count as f64).sqrt().ceil() as usize;
    let cols = count.div_ceil(rows);

    for r in 0..rows {
        for c in 0..cols {
            if positions.len() / 3 >= count {
                break;
            }
            let x = (c as f32 - cols as f32 / 2.0) * spread;
            let y = (r as f32 - rows as f32 / 2.0) * spread;
            let z = x * 0.3 + y * 0.1 + ((r * 7 + c * 13) % 100) as f32 * spread * 0.02;
            positions.push(x);
            positions.push(y);
            positions.push(z);
        }
    }
    positions
}

// ===========================================================================
// 10K points baseline
// ===========================================================================

#[test]
#[cfg(feature = "point-cloud")]
fn perf_10k_points_pipeline() {
    let n = 10_000;

    let t0 = Instant::now();
    let mut positions = generate_points(n, 1.0);
    let gen_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // Build Octree
    let t1 = Instant::now();
    let octree = Octree::build(&mut positions, 1000, 10);
    let octree_ms = t1.elapsed().as_secs_f64() * 1000.0;

    // Generate Tileset
    let t2 = Instant::now();
    let result = generate_tileset(&octree, &positions, None).expect("tileset failed");
    let tileset_ms = t2.elapsed().as_secs_f64() * 1000.0;

    eprintln!(
        "═══ 10K Pipeline ═══\n\
         Generate:  {:.2} ms\n\
         Octree:     {:.2} ms ({})\n\
         Tileset:    {:.2} ms ({} tiles, {} KB)\n\
         ════════════════════",
        gen_ms,
        octree_ms,
        octree.node_count(),
        tileset_ms,
        result.tile_count(),
        result.total_bytes() / 1024,
    );
}

// ===========================================================================
// 100K points pipeline
// ===========================================================================

#[test]
#[cfg(feature = "point-cloud")]
fn perf_100k_points_pipeline() {
    let n = 100_000;

    let t0 = Instant::now();
    let mut positions = generate_points(n, 0.5);
    let gen_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    let octree = Octree::build(&mut positions, 5000, 12);
    let octree_ms = t1.elapsed().as_secs_f64() * 1000.0;

    let t2 = Instant::now();
    let result = generate_tileset(&octree, &positions, None).expect("tileset failed");
    let tileset_ms = t2.elapsed().as_secs_f64() * 1000.0;

    // Validate tiles
    let t3 = Instant::now();
    for i in 0..result.tile_count() {
        let tile_data = result.tile(i as usize).unwrap();
        assert_eq!(&tile_data[0..4], b"pnts");
        let _ = parse_pnts_header(tile_data).unwrap();
    }
    let validate_ms = t3.elapsed().as_secs_f64() * 1000.0;

    eprintln!(
        "═══ 100K Pipeline ═══\n\
         Generate:  {:.2} ms\n\
         Octree:     {:.2} ms ({})\n\
         Tileset:    {:.2} ms ({} tiles, {:.2} MB)\n\
         Validate:   {:.2} ms\n\
         ════════════════════",
        gen_ms,
        octree_ms,
        octree.node_count(),
        tileset_ms,
        result.tile_count(),
        result.total_bytes() as f64 / 1_048_576.0,
        validate_ms,
    );
}

// ===========================================================================
// 1M points pipeline
// ===========================================================================

#[test]
#[cfg(feature = "point-cloud")]
fn perf_1m_points_pipeline() {
    let n = 1_000_000;

    let t0 = Instant::now();
    let mut positions = generate_points(n, 0.3);
    let gen_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    let octree = Octree::build(&mut positions, 10000, 14);
    let octree_ms = t1.elapsed().as_secs_f64() * 1000.0;

    let t2 = Instant::now();
    let result = generate_tileset(&octree, &positions, None).expect("tileset failed");
    let tileset_ms = t2.elapsed().as_secs_f64() * 1000.0;

    eprintln!(
        "═══ 1M Pipeline ═══\n\
         Generate:  {:.2} ms\n\
         Octree:     {:.2} ms ({})\n\
         Tileset:    {:.2} ms ({} tiles, {:.2} MB)\n\
         ════════════════════",
        gen_ms,
        octree_ms,
        octree.node_count(),
        tileset_ms,
        result.tile_count(),
        result.total_bytes() as f64 / 1_048_576.0,
    );
}

// ===========================================================================
// 10M points octree only
// ===========================================================================

#[test]
#[cfg(feature = "point-cloud")]
fn perf_10m_octree_build() {
    let n = 10_000_000;

    let t0 = Instant::now();
    let mut positions = generate_points(n, 0.2);
    let gen_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    let octree = Octree::build(&mut positions, 20000, 16);
    let octree_ms = t1.elapsed().as_secs_f64() * 1000.0;

    eprintln!(
        "═══ 10M Octree Build ═══\n\
         Generate:  {:.2} ms\n\
         Octree:     {:.2} ms ({})\n\
         ════════════════════════",
        gen_ms,
        octree_ms,
        octree.node_count(),
    );
}

// ===========================================================================
// LAZ roundtrip performance (requires laz-support)
// ===========================================================================

#[test]
#[cfg(feature = "laz-support")]
fn perf_laz_decompression_100k() {
    use laz::{LasZipCompressor, LazItemRecordBuilder, LazItemType, LazVlr};
    use wasm_spatial_core::parse_laz_points_core;

    let n = 100_000;
    let laz_items = LazItemRecordBuilder::new()
        .add_item(LazItemType::Point10)
        .build();
    let point_size = LazVlr::from_laz_items(laz_items.clone()).items_size();

    let raw: Vec<u8> = (0..n)
        .flat_map(|i| {
            let mut p = vec![0u8; point_size as usize];
            let x = (i as f64 * 0.1) as i32;
            let y = (i as f64 * 0.2).sin().to_degrees() as i32;
            let z = (i as f64 * 0.3).cos().to_degrees() as i32;
            p[0..4].copy_from_slice(&x.to_le_bytes());
            p[4..8].copy_from_slice(&y.to_le_bytes());
            p[8..12].copy_from_slice(&z.to_le_bytes());
            p
        })
        .collect();

    // Compress
    let t0 = Instant::now();
    let mut compressed = std::io::Cursor::new(Vec::new());
    {
        let mut compressor = LasZipCompressor::from_laz_items(&mut compressed, laz_items).unwrap();
        compressor.compress_many(&raw).unwrap();
        compressor.done().unwrap();
    }
    let compress_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let compressed_data = compressed.into_inner();

    eprintln!(
        "LAZ: {} → {} bytes ({:.1}% compression)",
        raw.len(),
        compressed_data.len(),
        compressed_data.len() as f64 / raw.len() as f64 * 100.0,
    );

    // Decompress (time only)
    let t1 = Instant::now();
    let _ = parse_laz_points_core(&compressed_data);
    let decomp_ms = t1.elapsed().as_secs_f64() * 1000.0;

    eprintln!(
        "═══ LAZ 100K ═══\n\
         Compress:   {:.2} ms\n\
         Decompress: {:.2} ms\n\
         ══════════════════",
        compress_ms, decomp_ms,
    );
}
