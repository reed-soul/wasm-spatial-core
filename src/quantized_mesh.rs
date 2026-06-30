//! Byte-exact Cesium quantized-mesh-1.0 encoder/decoder.
//!
//! Spec: https://github.com/CesiumGS/quantized-mesh
//! Module responsibility: (de)serialize the quantized-mesh binary stream.
//! Grid → mesh triangulation + geometry math lives here too.

// Helpers below are consumed by the assembler added in later tasks of the
// W3.6 plan. Allow dead code until the full encoder is wired up.
#![allow(dead_code)]

/// Zig-zag encode a signed delta into a non-negative integer.
/// Maps 0,-1,+1,-2,+2 → 0,1,2,3,4.
#[inline]
pub(crate) fn zigzag_encode(n: i32) -> u32 {
    ((n << 1) ^ (n >> 31)) as u32
}

/// Inverse of `zigzag_encode`.
#[inline]
pub(crate) fn zigzag_decode(n: u32) -> i32 {
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
pub(crate) fn decode_index_stream(encoded: &[u32]) -> Vec<u32> {
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

/// Build the fixed 88-byte quantized-mesh header.
///
/// `bs_center`/`bs_radius` define the bounding sphere (must contain all verts).
/// `horizon` may be `{0,0,0}` to disable horizon culling.
pub(crate) fn build_header(
    center_x: f64,
    center_y: f64,
    center_z: f64,
    min_height: f32,
    max_height: f32,
    bs_x: f64,
    bs_y: f64,
    bs_z: f64,
    bs_radius: f64,
    horizon_x: f64,
    horizon_y: f64,
    horizon_z: f64,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(88);
    buf.extend_from_slice(&center_x.to_le_bytes()); // 0..8
    buf.extend_from_slice(&center_y.to_le_bytes()); // 8..16
    buf.extend_from_slice(&center_z.to_le_bytes()); // 16..24
    buf.extend_from_slice(&min_height.to_le_bytes()); // 24..28  REAL f32
    buf.extend_from_slice(&max_height.to_le_bytes()); // 28..32  REAL f32
    buf.extend_from_slice(&bs_x.to_le_bytes()); // 32..40
    buf.extend_from_slice(&bs_y.to_le_bytes()); // 40..48
    buf.extend_from_slice(&bs_z.to_le_bytes()); // 48..56
    buf.extend_from_slice(&bs_radius.to_le_bytes()); // 56..64
    buf.extend_from_slice(&horizon_x.to_le_bytes()); // 64..72
    buf.extend_from_slice(&horizon_y.to_le_bytes()); // 72..80
    buf.extend_from_slice(&horizon_z.to_le_bytes()); // 80..88
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
        let header = build_header(
            0.0, 0.0, 0.0,    // center ECEF
            0.0_f32, 0.0_f32, // min/max height
            0.0, 0.0, 0.0,    // bounding sphere center
            1.0,              // bounding sphere radius
            0.0, 0.0, 0.0,    // horizon occlusion point (disabled)
        );
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
}
