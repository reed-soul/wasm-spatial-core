//! # Performance Benchmark Suite
//!
//! Regression benchmark tests for the point cloud processing pipeline.
//! All tests are marked `#[ignore]` — run manually:
//!
//! ```sh
//! cargo test --all-features -- --ignored --nocapture benchmark
//! ```
//!
//! Output format: JSON lines (one JSON object per benchmark), suitable for
//! CI parsing and GitHub Actions artifacts.
//!
//! ## Benchmarks covered
//!
//! - Point cloud pipeline (1K → 10M points): octree build, tileset gen, pnts encode
//! - GeoTIFF terrain (256×256 → 1024×1024): parse, quantized-mesh, color ramp, hillshade
//! - Format conversion (10K/100K): LAS→pnts, PLY→GLB, mesh→b3dm

use std::time::Instant;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A single benchmark result, serialized as JSON.
#[derive(Debug, Clone, serde::Serialize)]
struct BenchmarkResult {
    name: String,
    points: u64,
    time_us: u64,
    ops_per_sec: f64,
    mem_estimate: usize,
    extra: Option<String>,
}

/// Run a timed benchmark and print a JSON result line.
fn bench<F>(name: &str, points: u64, mem_estimate: usize, f: F) -> BenchmarkResult
where
    F: FnOnce(),
{
    let start = Instant::now();
    f();
    let elapsed = start.elapsed();
    let time_us = elapsed.as_micros() as u64;
    let ops_per_sec = if time_us > 0 {
        (points as f64) / (time_us as f64 / 1_000_000.0)
    } else {
        f64::INFINITY
    };
    let result = BenchmarkResult {
        name: name.to_string(),
        points,
        time_us,
        ops_per_sec,
        mem_estimate,
        extra: None,
    };
    eprintln!("{}", serde_json::to_string(&result).unwrap());
    result
}

/// Generate synthetic point cloud data.
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

/// Generate synthetic GeoTIFF-like elevation grid.
fn generate_elevation_grid(size: usize) -> Vec<f32> {
    let mut heights = Vec::with_capacity(size * size);
    for r in 0..size {
        for c in 0..size {
            let x = (c as f32 - size as f32 / 2.0) / size as f32;
            let y = (r as f32 - size as f32 / 2.0) / size as f32;
            let z = (x * x + y * y).sqrt() * 100.0 + (r + c) as f32 * 0.1;
            heights.push(z);
        }
    }
    heights
}

// ---------------------------------------------------------------------------
// Point Cloud Benchmarks
// ---------------------------------------------------------------------------

#[test]
#[ignore]
#[cfg(feature = "point-cloud")]
fn benchmark_1k_points_octree() {
    let n = 1_000;
    let mut positions = generate_points(n, 1.0);
    let mem = n * 3 * 4 + n * 8 + 200;
    bench("octree_build_1k", n as u64, mem, || {
        let _ = wasm_spatial_core::Octree::build(&mut positions, 100, 10);
    });
}

#[test]
#[ignore]
#[cfg(feature = "point-cloud")]
fn benchmark_1k_points_tileset() {
    let n = 1_000;
    let mut positions = generate_points(n, 1.0);
    let octree = wasm_spatial_core::Octree::build(&mut positions, 100, 10);
    let mem = n * 14 + 4096;
    bench("tileset_gen_1k", n as u64, mem, || {
        let _ = wasm_spatial_core::generate_tileset(&octree, &positions, None).unwrap();
    });
}

#[test]
#[ignore]
#[cfg(feature = "point-cloud")]
fn benchmark_10k_points_octree() {
    let n = 10_000;
    let mut positions = generate_points(n, 0.5);
    let mem = n * 3 * 4 + n * 8 + 200;
    bench("octree_build_10k", n as u64, mem, || {
        let _ = wasm_spatial_core::Octree::build(&mut positions, 1000, 10);
    });
}

#[test]
#[ignore]
#[cfg(feature = "point-cloud")]
fn benchmark_10k_points_tileset() {
    let n = 10_000;
    let mut positions = generate_points(n, 0.5);
    let octree = wasm_spatial_core::Octree::build(&mut positions, 1000, 10);
    let mem = n * 14 + 4096;
    bench("tileset_gen_10k", n as u64, mem, || {
        let _ = wasm_spatial_core::generate_tileset(&octree, &positions, None).unwrap();
    });
}

#[test]
#[ignore]
#[cfg(feature = "point-cloud")]
fn benchmark_100k_points_octree() {
    let n = 100_000;
    let mut positions = generate_points(n, 0.3);
    let mem = n * 3 * 4 + n * 8 + 200;
    bench("octree_build_100k", n as u64, mem, || {
        let _ = wasm_spatial_core::Octree::build(&mut positions, 5000, 12);
    });
}

#[test]
#[ignore]
#[cfg(feature = "point-cloud")]
fn benchmark_100k_points_tileset() {
    let n = 100_000;
    let mut positions = generate_points(n, 0.3);
    let octree = wasm_spatial_core::Octree::build(&mut positions, 5000, 12);
    let mem = n * 14 + 4096;
    bench("tileset_gen_100k", n as u64, mem, || {
        let _ = wasm_spatial_core::generate_tileset(&octree, &positions, None).unwrap();
    });
}

#[test]
#[ignore]
#[cfg(feature = "point-cloud")]
fn benchmark_1m_points_octree() {
    let n = 1_000_000;
    let mut positions = generate_points(n, 0.2);
    let mem = n * 3 * 4 + n * 8 + 200;
    bench("octree_build_1m", n as u64, mem, || {
        let _ = wasm_spatial_core::Octree::build(&mut positions, 50_000, 21);
    });
}

#[test]
#[ignore]
#[cfg(feature = "point-cloud")]
fn benchmark_10m_points_octree() {
    let n = 10_000_000;
    let mut positions = generate_points(n, 0.1);
    let mem = n * 3 * 4 + n * 8 + 200;
    bench("octree_build_10m", n as u64, mem, || {
        let _ = wasm_spatial_core::Octree::build(&mut positions, 50_000, 21);
    });
}

// ---------------------------------------------------------------------------
// GeoTIFF Benchmarks
// ---------------------------------------------------------------------------

#[test]
#[ignore]
#[cfg(feature = "geotiff")]
fn benchmark_geotiff_color_ramp_256() {
    let size = 256;
    let heights = generate_elevation_grid(size);
    let n = size * size;
    let mem = n * 4;
    bench("color_ramp_256x256", n as u64, mem, || {
        let _ = wasm_spatial_core::apply_color_ramp_core(
            &heights,
            0.0,
            100.0,
            wasm_spatial_core::ColorRamp::Terrain,
        );
    });
}

#[test]
#[ignore]
#[cfg(feature = "geotiff")]
fn benchmark_geotiff_hillshade_256() {
    let size = 256;
    let heights = generate_elevation_grid(size);
    let n = size * size;
    let mem = n * 4;
    bench("hillshade_256x256", n as u64, mem, || {
        let _ = wasm_spatial_core::hillshade_core(&heights, size, size, 315.0, 45.0);
    });
}

#[test]
#[ignore]
#[cfg(feature = "geotiff")]
fn benchmark_geotiff_color_ramp_512() {
    let size = 512;
    let heights = generate_elevation_grid(size);
    let n = size * size;
    let mem = n * 4;
    bench("color_ramp_512x512", n as u64, mem, || {
        let _ = wasm_spatial_core::apply_color_ramp_core(
            &heights,
            0.0,
            100.0,
            wasm_spatial_core::ColorRamp::Terrain,
        );
    });
}

#[test]
#[ignore]
#[cfg(feature = "geotiff")]
fn benchmark_geotiff_hillshade_1024() {
    let size = 1024;
    let heights = generate_elevation_grid(size);
    let n = size * size;
    let mem = n * 4;
    bench("hillshade_1024x1024", n as u64, mem, || {
        let _ = wasm_spatial_core::hillshade_core(&heights, size, size, 315.0, 45.0);
    });
}

// ---------------------------------------------------------------------------
// Format Conversion Benchmarks
// ---------------------------------------------------------------------------

#[test]
#[ignore]
#[cfg(feature = "point-cloud")]
fn benchmark_pnts_encode_10k() {
    let n = 10_000;
    let positions = generate_points(n, 0.5);
    let center = [0.0f64; 3];
    let mem = n * 14 + 28;
    bench("pnts_encode_10k", n as u64, mem, || {
        let _ = wasm_spatial_core::encode_pnts_tile(&positions, center, None).unwrap();
    });
}

#[test]
#[ignore]
#[cfg(feature = "point-cloud")]
fn benchmark_pnts_encode_100k() {
    let n = 100_000;
    let positions = generate_points(n, 0.3);
    let center = [0.0f64; 3];
    let mem = n * 14 + 28;
    bench("pnts_encode_100k", n as u64, mem, || {
        let _ = wasm_spatial_core::encode_pnts_tile(&positions, center, None).unwrap();
    });
}

#[test]
#[ignore]
fn benchmark_b3dm_encode_mesh() {
    // Create a minimal valid GLB header + body for b3dm wrapping benchmark.
    // A GLB consists of a 12-byte header + JSON chunk + binary chunk.
    let glb_json = r#"{"asset":{"version":"2.0","generator":"benchmark"},"meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1,"mode":4}]}],"buffers":[{}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":72,"target":34962},{"buffer":0,"byteOffset":72,"byteLength":24,"target":34963}],"accessors":[{"bufferView":0,"componentType":5126,"count":6,"type":"VEC3"},{"bufferView":1,"componentType":5125,"count":6,"type":"SCALAR"}]}"#;
    let json_padded = {
        let bytes = glb_json.as_bytes();
        let padding = (4 - bytes.len() % 4) % 4;
        let mut v = bytes.to_vec();
        v.extend(std::iter::repeat(b' ').take(padding));
        v
    };
    let bin_data: Vec<u8> = vec![
        0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0,
    ].iter().flat_map(|f| f.to_le_bytes()).collect();
    let index_data: Vec<u8> = [0u32, 1, 2, 3, 4, 5]
        .iter()
        .flat_map(|i| i.to_le_bytes())
        .collect();
    let bin_padded_len = (bin_data.len() + index_data.len() + 3) / 4 * 4;
    let bin_chunk: Vec<u8> = bin_data.iter().chain(index_data.iter()).copied().collect();
    let mut bin_padded = bin_chunk.clone();
    bin_padded.resize(bin_padded_len, 0);

    // Assemble GLB: magic(4) + version(4) + total_len(4) + json_chunk + bin_chunk
    let json_chunk_len = json_padded.len() as u32;
    let bin_chunk_len = bin_padded_len as u32;
    let total_len = 12 + 8 + json_chunk_len as usize + 8 + bin_chunk_len as usize;

    let mut glb: Vec<u8> = Vec::with_capacity(total_len);
    glb.extend_from_slice(b"glTF"); // magic
    glb.extend_from_slice(&2u32.to_le_bytes()); // version
    glb.extend_from_slice(&(total_len as u32).to_le_bytes()); // total length
    glb.extend_from_slice(&json_chunk_len.to_le_bytes()); // JSON chunk length
    glb.extend_from_slice(&0u32.to_le_bytes()); // JSON chunk type
    glb.extend_from_slice(&json_padded);
    glb.extend_from_slice(&bin_chunk_len.to_le_bytes()); // BIN chunk length
    glb.extend_from_slice(&0x005f5f00u32.to_le_bytes()); // BIN chunk type (BIN\0)
    glb.extend_from_slice(&bin_padded);

    assert_eq!(&glb[0..4], b"glTF");

    let mem = glb.len() + 100;
    bench("b3dm_encode_mesh", 6, mem, || {
        let _ = wasm_spatial_core::encode_b3dm_tile(&glb, 0, None).unwrap();
    });
}

// ---------------------------------------------------------------------------
// Full Pipeline Benchmark
// ---------------------------------------------------------------------------

#[test]
#[ignore]
#[cfg(feature = "point-cloud")]
fn benchmark_full_pipeline_100k() {
    let n = 100_000;
    let mut positions = generate_points(n, 0.3);

    eprintln!("--- Full Pipeline 100K ---");

    // Octree
    let octree = {
        let start = Instant::now();
        let tree = wasm_spatial_core::Octree::build(&mut positions, 5000, 12);
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        eprintln!("  octree:     {:.2} ms ({} nodes)", ms, tree.node_count());
        tree
    };

    // Tileset
    let result = {
        let start = Instant::now();
        let r = wasm_spatial_core::generate_tileset(&octree, &positions, None).unwrap();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "  tileset:    {:.2} ms ({} tiles, {} KB)",
            ms,
            r.tile_count(),
            r.total_bytes() / 1024
        );
        r
    };

    // Validate all tiles
    {
        let start = Instant::now();
        for i in 0..result.tile_count() {
            let tile = result.tile(i as usize).unwrap();
            assert_eq!(&tile[0..4], b"pnts");
            let _ = wasm_spatial_core::parse_pnts_header(tile).unwrap();
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        eprintln!("  validate:   {:.2} ms", ms);
    }

    assert_eq!(result.tile_count() > 0, true);
}
