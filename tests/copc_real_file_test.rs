//! Real-file COPC regression tests.
//!
//! Runs only when a real COPC sample is available locally (the 81 MB autzen
//! sample is too large to commit). Download once with:
//!
//! ```sh
//! curl -L -o /tmp/autzen.copc.laz \
//!   https://s3.amazonaws.com/hobu-lidar/autzen-classified.copc.laz
//! ```
//!
//! Without the file present these tests are skipped (not failed), so CI and
//! `cargo test` stay hermetic.

#[cfg(feature = "laz-support")]
#[test]
fn copc_real_file_end_to_end() {
    let path = std::path::Path::new("/tmp/autzen.copc.laz");
    if !path.exists() {
        eprintln!("skipping: /tmp/autzen.copc.laz not present");
        return;
    }
    let bytes = std::fs::read(path).unwrap();

    let info = wasm_spatial_core::parse_copc_header_core(&bytes).unwrap();
    assert_eq!(info.version_major, 1);
    assert_eq!(info.version_minor, 4);
    assert_eq!(info.point_count, 10_653_336);
    // ASPRS interleaved bounds (verified independently against the spec).
    assert!(
        (info.bounds.3 - 639_003.73).abs() < 0.01,
        "{:?}",
        info.bounds
    );
    assert!(
        (info.bounds.0 - 635_577.79).abs() < 0.01,
        "{:?}",
        info.bounds
    );
    assert!(
        (info.bounds.4 - 853_537.66).abs() < 0.01,
        "{:?}",
        info.bounds
    );
    assert!(
        (info.bounds.1 - 848_882.15).abs() < 0.01,
        "{:?}",
        info.bounds
    );
    assert!((info.bounds.5 - 615.26).abs() < 0.01, "{:?}", info.bounds);
    assert!((info.bounds.2 - 406.14).abs() < 0.01, "{:?}", info.bounds);
    // Real arithmetic-coded chunk table decodes to 278 chunks.
    assert_eq!(info.chunk_table.len(), 278);
    let total: u64 = info.chunk_table.iter().map(|c| c.1).sum();
    assert_eq!(total, info.point_count);
    assert!(info.copc_info.is_some(), "COPC info VLR must be found");

    let header_bytes = &bytes[..info.point_data_offset as usize];

    // First chunk.
    let first = info.chunk_table[0];
    let cloud = wasm_spatial_core::read_copc_chunk_core(
        &bytes,
        first.0,
        first.2,
        first.1 as usize,
        header_bytes,
    )
    .unwrap();
    assert_eq!(cloud.point_count() as u64, first.1);

    // LAST chunk — regression: previously panicked (capacity overflow).
    let last = *info.chunk_table.last().unwrap();
    let cloud = wasm_spatial_core::read_copc_chunk_core(
        &bytes,
        last.0,
        last.2,
        last.1 as usize,
        header_bytes,
    )
    .unwrap();
    assert_eq!(
        cloud.point_count() as u64,
        last.1,
        "last chunk must decompress"
    );

    // Standalone chunk decompression (streaming path): same chunk, but from
    // just its own bytes — no full file, no chunk table seek.
    let data_start = info.point_data_offset as usize + 8;
    let start = data_start + last.0 as usize;
    let end = start + last.2 as usize;
    let standalone = wasm_spatial_core::read_copc_chunk_standalone_core(
        &bytes[start..end],
        last.1 as usize,
        header_bytes,
    )
    .unwrap();
    assert_eq!(standalone.point_count() as u64, last.1);
    // Same first point as the full-file path.
    let a = standalone.positions_native();
    let b = cloud.positions_native();
    assert_eq!(a[0], b[0]);
    assert_eq!(a[1], b[1]);
    assert_eq!(a[2], b[2]);
}
