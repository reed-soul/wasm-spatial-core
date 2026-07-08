//! 50M-point LAZ end-to-end pipeline validation.
//!
//! Validates header parse → LAZ decompress → point parse → octree → tileset
//! at production scale (50 million points, ~200MB LAZ, ~600MB 3D Tiles output).

#![cfg(feature = "laz-support")]

use rand::Rng;
use std::io::Cursor;
use std::time::Instant;
use wasm_spatial_core::test_exports::test_helpers::{get_point_count, get_positions};
use wasm_spatial_core::{parse_las_header_core, parse_laz_points_core, Octree};

/// Build a valid LAZ blob from n uniformly distributed points in [0,1000]^3
/// using the laz crate's own LasZipCompressor (Point10 format).
fn build_laz(n: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let laz_items = laz::LazItemRecordBuilder::new()
        .add_item(laz::LazItemType::Point10)
        .build();
    let ps = laz::LazVlr::from_laz_items(laz_items.clone()).items_size() as u16;
    let mut raw = vec![0u8; n * ps as usize];
    for i in 0..n {
        let x = (rng.gen::<f64>() * 1000.0) as i32;
        let y = (rng.gen::<f64>() * 1000.0) as i32;
        let z = (rng.gen::<f64>() * 1000.0) as i32;
        let o = i * ps as usize;
        raw[o..o + 4].copy_from_slice(&x.to_le_bytes());
        raw[o + 4..o + 8].copy_from_slice(&y.to_le_bytes());
        raw[o + 8..o + 12].copy_from_slice(&z.to_le_bytes());
    }
    // Compress
    let mut comp = Cursor::new(Vec::new());
    {
        let mut c = laz::LasZipCompressor::from_laz_items(&mut comp, laz_items).unwrap();
        c.compress_many(&raw).unwrap();
        c.done().unwrap();
    }
    let cd = comp.into_inner();
    drop(raw);
    // Build LAS header + LASZIP VLR + compressed data
    let vlr = laz::LazVlr::from_laz_items(
        laz::LazItemRecordBuilder::new()
            .add_item(laz::LazItemType::Point10)
            .build(),
    );
    let vd = {
        let mut b = Cursor::new(Vec::new());
        vlr.write_to(&mut b).unwrap();
        b.into_inner()
    };
    let vt = (2 + 16 + 2 + 2 + 32) + vd.len();
    let hs: u32 = 230;
    let po = hs + vt as u32;
    let mut buf = vec![0u8; hs as usize];
    buf[0..4].copy_from_slice(b"LASF");
    buf[24] = 1;
    buf[25] = 2;
    buf[94..96].copy_from_slice(&(hs as u16).to_le_bytes());
    buf[96..100].copy_from_slice(&po.to_le_bytes());
    buf[100..104].copy_from_slice(&1u32.to_le_bytes()); // number of VLRs = 1 (LASZIP)
    buf[104] = 0x80;
    buf[105..107].copy_from_slice(&ps.to_le_bytes());
    buf[107..111].copy_from_slice(&(n as u32).to_le_bytes()); // num points (ASPRS offset 107)
    buf[131..139].copy_from_slice(&1.0f64.to_le_bytes());
    buf[139..147].copy_from_slice(&1.0f64.to_le_bytes());
    buf[147..155].copy_from_slice(&1.0f64.to_le_bytes());
    buf.resize(buf.len() + vt, 0);
    let vs = hs as usize;
    let mut uid = [0u8; 16];
    uid[..14].copy_from_slice(b"laszip encoded");
    buf[vs + 2..vs + 18].copy_from_slice(&uid);
    buf[vs + 18..vs + 20].copy_from_slice(&22204u16.to_le_bytes());
    buf[vs + 20..vs + 22].copy_from_slice(&(vd.len() as u16).to_le_bytes());
    buf[vs + 54..vs + vt].copy_from_slice(&vd);
    buf.extend_from_slice(&cd);
    buf
}

#[test]
fn test_50m_laz_header() {
    let laz = build_laz(50_000_000);
    let hdr = parse_las_header_core(&laz).unwrap();
    assert_eq!(hdr.num_points(), 50_000_000);
    assert_eq!(hdr.version_major(), 1);
    assert_eq!(hdr.version_minor(), 2);
    println!(
        "✅ 50M LAZ header: {} points, v{}.{}",
        hdr.num_points(),
        hdr.version_major(),
        hdr.version_minor()
    );
}

#[test]
#[ignore] // Too slow for CI (~200s debug). Run locally: cargo test --test large_laz_e2e --features "point-cloud test-helpers laz-support" --release -- --ignored --nocapture --test-threads=1
fn test_50m_laz_full_pipeline() {
    let n = 50_000_000;
    let t = Instant::now();

    // 1. Generate & compress
    let laz = build_laz(n);
    let gen = t.elapsed();
    println!(
        "  [Gen]      {:.1} MB LAZ in {:?}",
        laz.len() as f64 / 1e6,
        gen
    );

    // 2. Header
    let hdr = parse_las_header_core(&laz).unwrap();
    assert_eq!(hdr.num_points(), n as u32);

    // 3. Decompress
    let cloud = parse_laz_points_core(&laz).unwrap();
    let mut positions = get_positions(&cloud).to_vec();
    let count = get_point_count(&cloud);
    drop(laz);
    assert_eq!(count, n as u32);

    // 4. Octree
    let octree = Octree::build(&mut positions, 50000, 21);
    assert!(octree.leaf_count() > 1);

    // 5. Tileset
    let tileset = wasm_spatial_core::generate_tileset(&octree, &positions, None).expect("tileset");
    assert!(tileset.tile_count() > 0);

    println!("  [Decomp]   {} pts, {:?}", count, t.elapsed() - gen);
    println!(
        "  [Octree]   {} nodes, {} leaves",
        octree.node_count(),
        octree.leaf_count()
    );
    println!(
        "  [Tileset]  {} tiles, {:.1} MB",
        tileset.tile_count(),
        tileset.total_bytes() as f64 / 1e6
    );
    println!("  [Total]    {:?}", t.elapsed());
}
