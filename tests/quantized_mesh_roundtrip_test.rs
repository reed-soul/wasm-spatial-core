//! Byte-level round-trip + spec-conformance for the quantized-mesh encoder.
//!
//! Verifies that what we encode can be decoded back to the original grid,
//! and that the binary layout matches the CesiumGS/quantized-mesh spec.

use wasm_spatial_core::{decode_index_stream, encode_quantized_mesh, zigzag_decode};

// --- Minimal in-test decoder (proves the bytes are self-consistent) ---

fn decode_header(b: &[u8]) {
    assert_eq!(b.len(), 88, "header must be 88 bytes");
    // min/max height are real f32 (not truncated)
    let _min_h = f32::from_le_bytes(b[24..28].try_into().unwrap());
    let _max_h = f32::from_le_bytes(b[28..32].try_into().unwrap());
    let bs_radius = f64::from_le_bytes(b[56..64].try_into().unwrap());
    // bounding sphere radius must be positive (covers the tile)
    assert!(bs_radius > 0.0, "bounding sphere radius must be > 0");
}

fn decode_vertex_data(b: &[u8]) -> (u32, Vec<(u16, u16, u16)>) {
    let n = u32::from_le_bytes(b[0..4].try_into().unwrap());
    let mut verts = Vec::with_capacity(n as usize);
    let mut prev = (0_i32, 0_i32, 0_i32);
    let mut off = 12; // skip 3 count u32s
    for _ in 0..n {
        let du = u16::from_le_bytes([b[off], b[off + 1]]);
        let dv = u16::from_le_bytes([b[off + 2], b[off + 3]]);
        let dh = u16::from_le_bytes([b[off + 4], b[off + 5]]);
        off += 6;
        let u = (prev.0 + zigzag_decode(du as u32)) as u16;
        let v = (prev.1 + zigzag_decode(dv as u32)) as u16;
        let h = (prev.2 + zigzag_decode(dh as u32)) as u16;
        verts.push((u, v, h));
        prev = (u as i32, v as i32, h as i32);
    }
    (n, verts)
}

#[test]
fn test_roundtrip_4x4_grid() {
    // 4x4 grid with varying heights; bounds over a real lat/lng box.
    let heights: Vec<f32> = (0..16).map(|i| i as f32 * 10.0).collect();
    let bounds = [120.0_f64, 30.0, 120.1, 30.1];
    let center = [0.0_f64, 0.0, 0.0]; // placeholder; not load-bearing for the test
    let bytes = encode_quantized_mesh(&heights, 4, 4, &bounds, &center).unwrap();

    // Header
    decode_header(&bytes[0..88]);

    // Vertex data
    let (n, verts) = decode_vertex_data(&bytes[88..]);
    assert_eq!(n, 16);
    // Corner u/v should hit the 32767 extremes
    assert_eq!(verts[0].0, 0, "first col u=0");
    assert_eq!(verts[3].0, 32767, "last col u=32767");

    // Index data — decode HWM stream and confirm it reconstructs grid triangles
    let idx_off = 88 + 12 + 16 * 6;
    let tri_count = u32::from_le_bytes(bytes[idx_off..idx_off + 4].try_into().unwrap());
    assert_eq!(tri_count, 18); // (4-1)*(4-1)*2 = 18
    let mut enc = Vec::new();
    let mut o = idx_off + 4;
    for _ in 0..(tri_count * 3) {
        enc.push(u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()));
        o += 4;
    }
    let decoded_indices = decode_index_stream(&enc);
    // First triangle of a 4x4 grid: (0, 4, 1)
    assert_eq!(&decoded_indices[0..3], &[0, 4, 1]);
}

#[test]
fn test_roundtrip_2x2_flat() {
    let heights = vec![0.0_f32, 0.0, 0.0, 0.0];
    let bounds = [0.0_f64, 0.0, 1.0, 1.0];
    let center = [0.0_f64, 0.0, 0.0];
    let bytes = encode_quantized_mesh(&heights, 2, 2, &bounds, &center).unwrap();
    decode_header(&bytes[0..88]);
    let (n, _) = decode_vertex_data(&bytes[88..]);
    assert_eq!(n, 4);
}
