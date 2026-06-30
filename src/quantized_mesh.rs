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
}
