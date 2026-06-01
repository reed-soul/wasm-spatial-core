//! # Position Quantization Compression
//!
//! Simple lossy compression for 3D position data by quantizing Float32 → Uint16.
//! This achieves ~50% size reduction with minimal quality loss for bounded coordinates.
//!
//! ## Encoding
//!
//! 1. Compute axis-aligned bounding box (AABB) of all positions.
//! 2. For each coordinate, map from [min, max] → [0, 65535] (16-bit unsigned).
//! 3. Store as Uint16Array.
//!
//! ## Decoding
//!
//! 1. Map from [0, 65535] → [min, max].
//! 2. Store as Float32Array.
//!
//! ## Quality
//!
//! - 16-bit: ~0.003% relative error (sufficient for visualization)
//! - The original AABB is needed for reconstruction.

use wasm_bindgen::prelude::*;

// ===========================================================================
// Bounding Box
// ===========================================================================

/// Axis-aligned bounding box for quantization.
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min_x: f32,
    pub min_y: f32,
    pub min_z: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub max_z: f32,
}

/// WASM-exposed bounding box.
#[wasm_bindgen(js_name = "QuantBounds")]
#[derive(Debug, Clone)]
pub struct WasmQuantBounds {
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
}

#[wasm_bindgen]
impl WasmQuantBounds {
    /// Minimum X.
    #[wasm_bindgen(getter, js_name = "minX")]
    pub fn min_x(&self) -> f32 {
        self.min_x
    }

    /// Minimum Y.
    #[wasm_bindgen(getter, js_name = "minY")]
    pub fn min_y(&self) -> f32 {
        self.min_y
    }

    /// Minimum Z.
    #[wasm_bindgen(getter, js_name = "minZ")]
    pub fn min_z(&self) -> f32 {
        self.min_z
    }

    /// Maximum X.
    #[wasm_bindgen(getter, js_name = "maxX")]
    pub fn max_x(&self) -> f32 {
        self.max_x
    }

    /// Maximum Y.
    #[wasm_bindgen(getter, js_name = "maxY")]
    pub fn max_y(&self) -> f32 {
        self.max_y
    }

    /// Maximum Z.
    #[wasm_bindgen(getter, js_name = "maxZ")]
    pub fn max_z(&self) -> f32 {
        self.max_z
    }
}

impl From<BoundingBox> for WasmQuantBounds {
    fn from(b: BoundingBox) -> Self {
        WasmQuantBounds {
            min_x: b.min_x,
            min_y: b.min_y,
            min_z: b.min_z,
            max_x: b.max_x,
            max_y: b.max_y,
            max_z: b.max_z,
        }
    }
}

impl From<WasmQuantBounds> for BoundingBox {
    fn from(b: WasmQuantBounds) -> Self {
        BoundingBox {
            min_x: b.min_x,
            min_y: b.min_y,
            min_z: b.min_z,
            max_x: b.max_x,
            max_y: b.max_y,
            max_z: b.max_z,
        }
    }
}

// ===========================================================================
// Core functions
// ===========================================================================

/// Compute the bounding box of a position buffer.
fn compute_bounds(positions: &[f32]) -> BoundingBox {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut max_z = f32::NEG_INFINITY;

    for chunk in positions.chunks_exact(3) {
        min_x = min_x.min(chunk[0]);
        min_y = min_y.min(chunk[1]);
        min_z = min_z.min(chunk[2]);
        max_x = max_x.max(chunk[0]);
        max_y = max_y.max(chunk[1]);
        max_z = max_z.max(chunk[2]);
    }

    BoundingBox {
        min_x,
        min_y,
        min_z,
        max_x,
        max_y,
        max_z,
    }
}

/// Quantize Float32 positions to Uint16 using the given bounding box.
///
/// # Arguments
/// * `positions` — Flat `[x, y, z, ...]` Float32 positions.
/// * `bounds` — Bounding box for coordinate mapping.
/// * `bits` — Quantization bits (8, 16, or 32). 16 is recommended.
///
/// # Returns
/// Quantized positions as Uint16Array.
pub fn quantize_positions_core(positions: &[f32], bounds: &BoundingBox, bits: u32) -> Vec<u16> {
    let max_val = (1u32 << bits) - 1;
    let dx = bounds.max_x - bounds.min_x;
    let dy = bounds.max_y - bounds.min_y;
    let dz = bounds.max_z - bounds.min_z;

    positions
        .chunks_exact(3)
        .flat_map(|chunk| {
            let qx = if dx.abs() < 1e-10 {
                0
            } else {
                (((chunk[0] - bounds.min_x) / dx * max_val as f32) as u32).min(max_val) as u16
            };
            let qy = if dy.abs() < 1e-10 {
                0
            } else {
                (((chunk[1] - bounds.min_y) / dy * max_val as f32) as u32).min(max_val) as u16
            };
            let qz = if dz.abs() < 1e-10 {
                0
            } else {
                (((chunk[2] - bounds.min_z) / dz * max_val as f32) as u32).min(max_val) as u16
            };
            [qx, qy, qz]
        })
        .collect()
}

/// Dequantize Uint16 positions back to Float32 using the given bounding box.
///
/// # Arguments
/// * `quantized` — Flat quantized `[x, y, z, ...]` Uint16 positions.
/// * `bounds` — Original bounding box for coordinate mapping.
/// * `bits` — Quantization bits (must match the quantization step).
///
/// # Returns
/// Dequantized positions as Float32Array.
pub fn dequantize_positions_core(quantized: &[u16], bounds: &BoundingBox, bits: u32) -> Vec<f32> {
    let max_val = (1u32 << bits) - 1;
    let dx = bounds.max_x - bounds.min_x;
    let dy = bounds.max_y - bounds.min_y;
    let dz = bounds.max_z - bounds.min_z;

    quantized
        .chunks_exact(3)
        .flat_map(|chunk| {
            let x = bounds.min_x + chunk[0] as f32 / max_val as f32 * dx;
            let y = bounds.min_y + chunk[1] as f32 / max_val as f32 * dy;
            let z = bounds.min_z + chunk[2] as f32 / max_val as f32 * dz;
            [x, y, z]
        })
        .collect()
}

// ===========================================================================
// WASM exports
// ===========================================================================

/// Quantize Float32 positions to Uint16, returning both the quantized data
/// and the bounding box needed for reconstruction.
///
/// # Arguments
/// * `positions` — Flat `[x, y, z, ...]` Float32 positions.
/// * `bits` — Quantization bits (8 or 16). Default: 16.
///
/// # Returns
/// An object with `quantized` (Uint16Array) and `bounds` (QuantBounds).
#[wasm_bindgen(js_name = "quantizePositions")]
pub fn quantize_positions_js(positions: &[f32], bits: Option<u32>) -> QuantizeResult {
    let bits = bits.unwrap_or(16).clamp(8, 16);
    let bounds = compute_bounds(positions);
    let quantized = quantize_positions_core(positions, &bounds, bits);

    QuantizeResult {
        quantized,
        bounds: WasmQuantBounds::from(bounds),
    }
}

/// WASM result object for quantization.
#[wasm_bindgen]
pub struct QuantizeResult {
    quantized: Vec<u16>,
    bounds: WasmQuantBounds,
}

#[wasm_bindgen]
impl QuantizeResult {
    /// Quantized positions as Uint16Array.
    #[wasm_bindgen(getter)]
    pub fn quantized(&self) -> js_sys::Uint16Array {
        let arr = js_sys::Uint16Array::new_with_length(self.quantized.len() as u32);
        arr.copy_from(&self.quantized);
        arr
    }

    /// Bounding box for reconstruction.
    #[wasm_bindgen(getter)]
    pub fn bounds(&self) -> WasmQuantBounds {
        self.bounds.clone()
    }
}

/// Dequantize Uint16 positions back to Float32.
///
/// # Arguments
/// * `quantized` — Quantized positions (Uint16Array).
/// * `bounds` — Bounding box from quantization.
/// * `bits` — Quantization bits (must match).
///
/// # Returns
/// Float32Array of reconstructed positions.
#[wasm_bindgen(js_name = "dequantizePositions")]
pub fn dequantize_positions_js(
    quantized: &[u16],
    bounds: &WasmQuantBounds,
    bits: Option<u32>,
) -> js_sys::Float32Array {
    let bits = bits.unwrap_or(16);
    let bounds = BoundingBox::from(bounds.clone());
    let result = dequantize_positions_core(quantized, &bounds, bits);
    let arr = js_sys::Float32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    arr
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize_roundtrip() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0];
        let bounds = compute_bounds(&positions);
        let quantized = quantize_positions_core(&positions, &bounds, 16);
        let dequantized = dequantize_positions_core(&quantized, &bounds, 16);

        assert_eq!(quantized.len(), 12);
        assert_eq!(dequantized.len(), 12);

        // Roundtrip error should be very small
        for i in 0..positions.len() {
            let error = (positions[i] - dequantized[i]).abs();
            assert!(error < 0.001, "Position {} error too large: {}", i, error);
        }
    }

    #[test]
    fn test_quantize_8bit() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 100.0, 100.0, 100.0];
        let bounds = compute_bounds(&positions);
        let quantized = quantize_positions_core(&positions, &bounds, 8);
        let dequantized = dequantize_positions_core(&quantized, &bounds, 8);

        assert_eq!(quantized.len(), 6);
        // 8-bit is coarser — allow larger error
        for i in 0..positions.len() {
            let error = (positions[i] - dequantized[i]).abs();
            assert!(error < 1.0, "8-bit error too large at {}: {}", i, error);
        }
    }

    #[test]
    fn test_quantize_empty() {
        let positions: Vec<f32> = vec![];
        let bounds = compute_bounds(&positions);
        let quantized = quantize_positions_core(&positions, &bounds, 16);
        assert!(quantized.is_empty());
    }

    #[test]
    fn test_quantize_single_point() {
        let positions: Vec<f32> = vec![42.0, -17.5, std::f32::consts::PI];
        let bounds = compute_bounds(&positions);
        let quantized = quantize_positions_core(&positions, &bounds, 16);
        assert_eq!(quantized.len(), 3);

        // Single point should roundtrip exactly
        let dequantized = dequantize_positions_core(&quantized, &bounds, 16);
        assert_eq!(positions, dequantized);
    }

    #[test]
    fn test_quantize_negative_coords() {
        let positions: Vec<f32> = vec![-100.0, -200.0, -300.0, 100.0, 200.0, 300.0, 0.0, 0.0, 0.0];
        let bounds = compute_bounds(&positions);
        assert_eq!(bounds.min_x, -100.0);
        assert_eq!(bounds.max_x, 100.0);

        let quantized = quantize_positions_core(&positions, &bounds, 16);
        let dequantized = dequantize_positions_core(&quantized, &bounds, 16);

        for i in 0..positions.len() {
            let error = (positions[i] - dequantized[i]).abs();
            assert!(error < 0.01, "Negative coord error at {}: {}", i, error);
        }
    }

    #[test]
    fn test_size_reduction() {
        // 1000 points: Float32 = 12KB, Uint16 = 6KB → 50% reduction
        let n = 1000;
        let positions: Vec<f32> = (0..n)
            .flat_map(|i| [i as f32, i as f32 * 0.5, i as f32 * 0.3])
            .collect();
        let bounds = compute_bounds(&positions);
        let quantized = quantize_positions_core(&positions, &bounds, 16);

        let f32_bytes = positions.len() * 4; // Float32
        let u16_bytes = quantized.len() * 2; // Uint16
        assert_eq!(u16_bytes, f32_bytes / 2);
    }
}
