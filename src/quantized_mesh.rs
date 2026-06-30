//! Byte-exact Cesium quantized-mesh-1.0 encoder/decoder.
//!
//! Spec: https://github.com/CesiumGS/quantized-mesh
//! Module responsibility: (de)serialize the quantized-mesh binary stream.
//! Grid → mesh triangulation + geometry math lives here too.

// decode helpers are pub (#[doc(hidden)]) so the round-trip integration test
// can exercise them; all other helpers are pub(crate) and consumed by the
// assembler below.
#![allow(dead_code)]

/// Zig-zag encode a signed delta into a non-negative integer.
/// Maps 0,-1,+1,-2,+2 → 0,1,2,3,4.
#[inline]
pub(crate) fn zigzag_encode(n: i32) -> u32 {
    ((n << 1) ^ (n >> 31)) as u32
}

/// Inverse of `zigzag_encode`.
#[inline]
#[doc(hidden)]
pub fn zigzag_decode(n: u32) -> i32 {
    ((n >> 1) as i32) ^ -((n & 1) as i32)
}

/// Encode an index stream with the high-water-mark scheme:
/// if `index > highWaterMark`, emit zigzag(-(index - hwm)) and raise hwm;
/// else emit zigzag(hwm - index).
pub(crate) fn encode_index_stream(indices: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(indices.len());
    let mut hwm: i64 = 0;
    for &idx in indices {
        let signed = idx as i64;
        if signed > hwm {
            out.push(zigzag_encode(-(signed - hwm) as i32));
            hwm = signed;
        } else {
            out.push(zigzag_encode((hwm - signed) as i32));
        }
    }
    out
}

/// Inverse of `encode_index_stream`.
#[doc(hidden)]
pub fn decode_index_stream(encoded: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(encoded.len());
    let mut hwm: i64 = 0;
    for &e in encoded {
        let d = zigzag_decode(e) as i64;
        // d<0 path = "index grew": next = hwm + (-d) = hwm - d
        // d>=0 path = "index shrank": next = hwm - d
        let next = hwm - d;
        out.push(next as u32);
        // The encoder's watermark is monotonic (only rises); mirror that here.
        if next > hwm {
            hwm = next;
        }
    }
    out
}

// ===========================================================================
// Header (Task 2) — fixed 88-byte layout per CesiumGS/quantized-mesh spec
// ===========================================================================

/// Fields for the 88-byte quantized-mesh header.
pub(crate) struct MeshHeader {
    pub center: [f64; 3],
    pub min_height: f32,
    pub max_height: f32,
    /// Bounding sphere center (ECEF) + radius (must contain all verts).
    pub bs_center: [f64; 3],
    pub bs_radius: f64,
    /// Horizon occlusion point; `{0,0,0}` disables horizon culling.
    pub horizon: [f64; 3],
}

/// Build the fixed 88-byte quantized-mesh header.
pub(crate) fn build_header(h: &MeshHeader) -> Vec<u8> {
    let mut buf = Vec::with_capacity(88);
    buf.extend_from_slice(&h.center[0].to_le_bytes()); // 0..8
    buf.extend_from_slice(&h.center[1].to_le_bytes()); // 8..16
    buf.extend_from_slice(&h.center[2].to_le_bytes()); // 16..24
    buf.extend_from_slice(&h.min_height.to_le_bytes()); // 24..28  REAL f32
    buf.extend_from_slice(&h.max_height.to_le_bytes()); // 28..32  REAL f32
    buf.extend_from_slice(&h.bs_center[0].to_le_bytes()); // 32..40
    buf.extend_from_slice(&h.bs_center[1].to_le_bytes()); // 40..48
    buf.extend_from_slice(&h.bs_center[2].to_le_bytes()); // 48..56
    buf.extend_from_slice(&h.bs_radius.to_le_bytes()); // 56..64
    buf.extend_from_slice(&h.horizon[0].to_le_bytes()); // 64..72
    buf.extend_from_slice(&h.horizon[1].to_le_bytes()); // 72..80
    buf.extend_from_slice(&h.horizon[2].to_le_bytes()); // 80..88
    buf
}

// ===========================================================================
// Vertex data (Task 3) — zig-zag delta encoded u/v/h, 32767 max
// ===========================================================================

const MAX_UV: u16 = 32767;

/// Build the VertexData block: counts + zig-zag-delta-encoded u/v/h.
///
/// Each of u, v, h has `vertex_count` entries. Deltas are computed against the
/// previous vertex of the same channel (first vertex deltas from 0).
pub(crate) fn build_vertex_data(u: &[u16], v: &[u16], h: &[u16]) -> Vec<u8> {
    let n = u.len() as u32;
    let mut buf = Vec::with_capacity(12 + n as usize * 6);
    buf.extend_from_slice(&n.to_le_bytes()); // vertexCount
    buf.extend_from_slice(&n.to_le_bytes()); // uVertexCount (legacy redundancy)
    buf.extend_from_slice(&n.to_le_bytes()); // vVertexCount

    let mut prev_u: i32 = 0;
    let mut prev_v: i32 = 0;
    let mut prev_h: i32 = 0;
    for i in 0..n as usize {
        let cu = u[i] as i32;
        let cv = v[i] as i32;
        let ch = h[i] as i32;
        let du = zigzag_encode(cu - prev_u) as u16;
        let dv = zigzag_encode(cv - prev_v) as u16;
        let dh = zigzag_encode(ch - prev_h) as u16;
        buf.extend_from_slice(&du.to_le_bytes());
        buf.extend_from_slice(&dv.to_le_bytes());
        buf.extend_from_slice(&dh.to_le_bytes());
        prev_u = cu;
        prev_v = cv;
        prev_h = ch;
    }
    buf
}

// ===========================================================================
// Index + edge blocks (Task 4) — high-water-mark encoded
// ===========================================================================

/// Build the IndexData block: triangleCount + HWM-encoded indices (u32 each).
pub(crate) fn build_index_block(triangles: &[u32]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + triangles.len() * 4);
    buf.extend_from_slice(&(triangles.len() as u32 / 3).to_le_bytes()); // triangleCount
    for enc in encode_index_stream(triangles) {
        buf.extend_from_slice(&enc.to_le_bytes());
    }
    buf
}

/// Build the four EdgeIndices blocks (west, south, east, north), HWM-encoded.
pub(crate) fn build_edge_indices_block(
    west: &[u32],
    south: &[u32],
    east: &[u32],
    north: &[u32],
) -> Vec<u8> {
    let mut buf = Vec::new();
    for edge in [west, south, east, north] {
        buf.extend_from_slice(&(edge.len() as u32).to_le_bytes());
        for enc in encode_index_stream(edge) {
            buf.extend_from_slice(&enc.to_le_bytes());
        }
    }
    buf
}

// ===========================================================================
// Grid geometry (Task 5) — triangulation, quantization, bounding sphere
// ===========================================================================

/// Triangulate a regular `width`×`height` grid into a CCW index list.
/// For each quad (col,row) emit (i0, i2, i1) and (i1, i2, i3) per spec winding.
pub(crate) fn grid_to_triangles(width: u32, height: u32) -> Vec<u32> {
    let mut tris = Vec::with_capacity(((width - 1) * (height - 1) * 6) as usize);
    for row in 0..height - 1 {
        for col in 0..width - 1 {
            let i0 = row * width + col;
            let i1 = i0 + 1;
            let i2 = (row + 1) * width + col;
            let i3 = i2 + 1;
            tris.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }
    tris
}

/// Quantize a grid coordinate index to the [0, 32767] range.
pub(crate) fn quantize_uv(idx: u32, dim: u32) -> u16 {
    if dim <= 1 {
        0
    } else {
        ((idx as f64 / (dim - 1) as f64) * MAX_UV as f64) as u16
    }
}

/// Quantize a height into [0, 32767] over [min_h, max_h].
pub(crate) fn quantize_height(h: f32, min_h: f32, max_h: f32) -> u16 {
    let range = max_h - min_h;
    if range <= 0.0 {
        0
    } else {
        ((h - min_h) / range * MAX_UV as f32) as u16
    }
}

/// Compute a bounding sphere (centroid + max-distance radius) in ECEF.
pub(crate) fn bounding_sphere(verts_ecef: &[[f64; 3]]) -> (f64, f64, f64, f64) {
    let n = verts_ecef.len() as f64;
    let (mut cx, mut cy, mut cz) = (0.0_f64, 0.0_f64, 0.0_f64);
    for [x, y, z] in verts_ecef {
        cx += x;
        cy += y;
        cz += z;
    }
    cx /= n;
    cy /= n;
    cz /= n;
    let mut r = 0.0_f64;
    for [x, y, z] in verts_ecef {
        let d = ((x - cx).powi(2) + (y - cy).powi(2) + (z - cz).powi(2)).sqrt();
        if d > r {
            r = d;
        }
    }
    (cx, cy, cz, r)
}

// ===========================================================================
// Top-level assembler (Task 6)
// ===========================================================================

/// Encode a height grid into a spec-conformant quantized-mesh-1.0 byte stream.
///
/// `bounds` = [min_lng, min_lat, max_lng, max_lat].
/// `center` = ECEF sphere-center (any reasonable centroid; affects culling only).
/// Returns the concatenated header + vertex + index + edge bytes.
pub fn encode_quantized_mesh(
    heights: &[f32],
    width: u32,
    height: u32,
    bounds: &[f64; 4],
    center: &[f64; 3],
) -> Result<Vec<u8>, String> {
    if width < 2 || height < 2 {
        return Err("QuantizedMesh: grid must be at least 2×2".into());
    }
    if heights.len() != (width * height) as usize {
        return Err(format!(
            "QuantizedMesh: heights length {} != width×height {}",
            heights.len(),
            width * height
        ));
    }

    // Height range
    let mut min_h = f32::INFINITY;
    let mut max_h = f32::NEG_INFINITY;
    for &h in heights {
        min_h = min_h.min(h);
        max_h = max_h.max(h);
    }

    // Quantize u/v/h per vertex (row-major, matching grid_to_triangles indexing)
    let mut u = Vec::with_capacity(heights.len());
    let mut v = Vec::with_capacity(heights.len());
    let mut h_q = Vec::with_capacity(heights.len());
    for row in 0..height {
        for col in 0..width {
            u.push(quantize_uv(col, width));
            v.push(quantize_uv(row, height));
            h_q.push(quantize_height(
                heights[(row * width + col) as usize],
                min_h,
                max_h,
            ));
        }
    }

    // Triangles + edge index lists
    let tris = grid_to_triangles(width, height);
    let west: Vec<u32> = (0..height).map(|r| r * width).collect();
    let south: Vec<u32> = (0..width).map(|c| (height - 1) * width + c).collect();
    let east: Vec<u32> = (0..height).rev().map(|r| r * width + width - 1).collect();
    let north: Vec<u32> = (0..width).rev().collect();

    // Bounding sphere: approximate ECEF verts via bounds corners at min/max height.
    let verts_ecef = approximate_ecef_corners(bounds, min_h, max_h);
    let (bsx, bsy, bsz, bsr) = bounding_sphere(&verts_ecef);

    let header = MeshHeader {
        center: [center[0], center[1], center[2]],
        min_height: min_h,
        max_height: max_h,
        bs_center: [bsx, bsy, bsz],
        bs_radius: bsr,
        horizon: [0.0, 0.0, 0.0], // horizon occlusion disabled
    };
    let mut buf = build_header(&header);
    buf.extend_from_slice(&build_vertex_data(&u, &v, &h_q));
    buf.extend_from_slice(&build_index_block(&tris));
    buf.extend_from_slice(&build_edge_indices_block(&west, &south, &east, &north));
    Ok(buf)
}

/// 8 ECEF corners of the tile's lat/lng box at min and max height (for bounding sphere).
fn approximate_ecef_corners(bounds: &[f64; 4], min_h: f32, max_h: f32) -> Vec<[f64; 3]> {
    let mut out = Vec::with_capacity(8);
    for &lng in &[bounds[0], bounds[2]] {
        for &lat in &[bounds[1], bounds[3]] {
            for &alt in &[min_h as f64, max_h as f64] {
                let (x, y, z) = crate::cesium_adapter::wgs84_to_cartesian3_single(lng, lat, alt);
                out.push([x, y, z]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zigzag_encode_roundtrip() {
        // zig-zag maps 0,-1,+1,-2,+2 → 0,1,2,3,4
        assert_eq!(zigzag_encode(0), 0);
        assert_eq!(zigzag_encode(-1), 1);
        assert_eq!(zigzag_encode(1), 2);
        assert_eq!(zigzag_encode(-2), 3);
        assert_eq!(zigzag_encode(2), 4);
        // Full round-trip over a representative range
        for v in [-5000_i32, -1, 0, 1, 32767] {
            assert_eq!(zigzag_decode(zigzag_encode(v)), v, "failed at v={}", v);
        }
    }

    #[test]
    fn test_high_water_mark_sequence() {
        // Encode a known index sequence and verify it reproduces the same values.
        let indices: Vec<u32> = vec![0, 1, 2, 0, 3, 1, 4];
        let encoded = encode_index_stream(&indices);
        let decoded = decode_index_stream(&encoded);
        assert_eq!(decoded, indices);
    }

    #[test]
    fn test_header_is_88_bytes_and_decodes() {
        let h = MeshHeader {
            center: [0.0, 0.0, 0.0],
            min_height: 0.0_f32,
            max_height: 0.0_f32,
            bs_center: [0.0, 0.0, 0.0],
            bs_radius: 1.0,
            horizon: [0.0, 0.0, 0.0],
        };
        let header = build_header(&h);
        assert_eq!(header.len(), 88, "header must be exactly 88 bytes");
        // min height at offset 24..28 is a real f32 (not truncated)
        let min_h = f32::from_le_bytes([header[24], header[25], header[26], header[27]]);
        assert_eq!(min_h, 0.0);
        // radius at offset 56..64
        let radius = f64::from_le_bytes(header[56..64].try_into().unwrap());
        assert_eq!(radius, 1.0);
    }

    #[test]
    fn test_vertex_data_uses_32767_max_and_zigzag_delta() {
        // 2x2 grid: u,v in {0, 32767}; heights all quantized to 0 (flat).
        let u: Vec<u16> = vec![0, 32767, 0, 32767];
        let v: Vec<u16> = vec![0, 0, 32767, 32767];
        let h: Vec<u16> = vec![0, 0, 0, 0];
        let bytes = build_vertex_data(&u, &v, &h);
        // vertexCount(4) + uVertexCount(4) + vVertexCount(4) = 12 header bytes
        assert_eq!(bytes.len(), 12 + 4 * 6); // 12 + 4 verts * 3 u16
                                             // First u delta: prev=0, encoded zigzag(0-0)=0
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 0);
        // Second vertex's u: 32767-0=32767, zigzag(32767) = 65534
        assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 65534);
    }

    #[test]
    fn test_index_block_roundtrip() {
        // Triangulate a 2x2 grid → 2 triangles, indices [0,2,1, 1,2,3].
        let tris: Vec<u32> = vec![0, 2, 1, 1, 2, 3];
        let block = build_index_block(&tris);
        // triangleCount(4) + 6 × u32
        assert_eq!(block.len(), 4 + 6 * 4);
        let n = u32::from_le_bytes(block[0..4].try_into().unwrap());
        assert_eq!(n, 2); // 6 indices / 3 = 2 triangles
        let encoded: Vec<u32> = block[4..]
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(decode_index_stream(&encoded), tris);
    }

    #[test]
    fn test_edge_indices_block_order() {
        // west=2 verts [0,2], south=2 [2,3], east=2 [3,1], north=2 [1,0]
        let edges = build_edge_indices_block(&[0_u32, 2], &[2, 3], &[3, 1], &[1, 0]);
        // 4 counts (16 bytes) + 8 indices (32 bytes)
        assert_eq!(edges.len(), 16 + 8 * 4);
        let west_cnt = u32::from_le_bytes(edges[0..4].try_into().unwrap());
        assert_eq!(west_cnt, 2);
        let north_cnt = u32::from_le_bytes(edges[12..16].try_into().unwrap());
        assert_eq!(north_cnt, 2);
    }

    #[test]
    fn test_grid_to_mesh_2x2() {
        let tris = grid_to_triangles(2, 2);
        assert_eq!(tris, vec![0, 2, 1, 1, 2, 3]);
    }

    #[test]
    fn test_quantize_to_32767() {
        assert_eq!(quantize_uv(0, 2), 0);
        assert_eq!(quantize_uv(1, 2), 32767);
        let q = quantize_height(50.0, 0.0, 100.0);
        assert!((16380..=16390).contains(&q), "got {}", q);
    }

    #[test]
    fn test_bounding_sphere_contains_all() {
        let verts_ecef = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let (cx, cy, cz, r) = bounding_sphere(&verts_ecef);
        for [x, y, z] in &verts_ecef {
            let d = ((x - cx).powi(2) + (y - cy).powi(2) + (z - cz).powi(2)).sqrt();
            assert!(d <= r + 1e-6, "vertex outside sphere: d={}, r={}", d, r);
        }
    }

    #[test]
    fn test_encode_produces_spec_layout() {
        // 2x2 flat grid, heights all 0, bounds [0,0,1,1], center ECEF origin.
        let heights = vec![0.0_f32, 0.0, 0.0, 0.0];
        let bounds = [0.0_f64, 0.0, 1.0, 1.0];
        let center = [0.0_f64, 0.0, 0.0];
        let bytes = encode_quantized_mesh(&heights, 2, 2, &bounds, &center).unwrap();
        // Layout: 88 header + 12 vertex counts + 4*6 verts + 4 tri-count + 6*4 idx
        //       + edge block (16 + 8*4)
        let expected = 88 + 12 + 24 + 4 + 24 + 16 + 32;
        assert_eq!(
            bytes.len(),
            expected,
            "len={}, header starts {:?}",
            bytes.len(),
            &bytes[0..8]
        );
        // Header sanity: center[0] f64 == 0.0
        assert_eq!(f64::from_le_bytes(bytes[0..8].try_into().unwrap()), 0.0);
    }
}
