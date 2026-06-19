//! Core runtime utilities — job memory budget and reusable buffers (Wave 1.3).

use wasm_bindgen::prelude::*;

#[cfg(feature = "point-cloud")]
use crate::point_cloud::estimate_memory_for_points;

/// Reusable buffer arena for multi-step point cloud pipelines.
///
/// Retains capacity across `clear()` calls to avoid repeated WASM heap growth.
#[derive(Debug, Default)]
pub struct ProcessingContext {
    positions: Vec<f32>,
    colors: Vec<u8>,
}

impl ProcessingContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-reserve position/color capacity for an upcoming job.
    pub fn reserve(&mut self, point_count: usize, has_color: bool) {
        self.positions.reserve(point_count.saturating_mul(3));
        if has_color {
            self.colors.reserve(point_count.saturating_mul(3));
        }
    }

    /// Mutable positions buffer (e.g. LAS parse target).
    pub fn positions_mut(&mut self) -> &mut Vec<f32> {
        &mut self.positions
    }

    /// Mutable colors buffer.
    pub fn colors_mut(&mut self) -> &mut Vec<u8> {
        &mut self.colors
    }

    /// Positions slice after a step completes.
    pub fn positions(&self) -> &[f32] {
        &self.positions
    }

    /// Colors slice (may be empty).
    pub fn colors(&self) -> &[u8] {
        &self.colors
    }

    /// Clear logical length while keeping capacity for the next job step.
    pub fn clear(&mut self) {
        self.positions.clear();
        self.colors.clear();
    }

    /// Total reserved bytes (positions + colors).
    pub fn reserved_bytes(&self) -> usize {
        self.positions.capacity() * std::mem::size_of::<f32>()
            + self.colors.capacity() * std::mem::size_of::<u8>()
    }
}

/// High-level pipeline operation for memory estimation.
#[derive(Debug, Clone, Copy)]
pub enum JobOp {
    LasParse { point_count: u32, has_color: bool },
    OctreeBuild { point_count: u32 },
    TilesetGenerate { point_count: u32, leaf_count: u32 },
}

/// Estimate peak bytes for a pipeline step (heuristic; typically within 2× actual).
pub fn estimate_job_bytes(op: JobOp) -> usize {
    match op {
        JobOp::LasParse {
            point_count,
            has_color,
        } => {
            #[cfg(feature = "point-cloud")]
            {
                estimate_memory_for_points(point_count as usize, has_color, false)
            }
            #[cfg(not(feature = "point-cloud"))]
            {
                let _ = (point_count, has_color);
                0
            }
        }
        JobOp::OctreeBuild { point_count } => {
            // Positions (f32×3) + octree node overhead (~32 B per 8k points heuristic)
            let pos = point_count as usize * 3 * std::mem::size_of::<f32>();
            let nodes = (point_count as usize / 8_000 + 1) * 64;
            pos + nodes
        }
        JobOp::TilesetGenerate {
            point_count,
            leaf_count,
        } => {
            let pos = point_count as usize * 3 * std::mem::size_of::<f32>();
            let tiles = leaf_count as usize * 4_096; // ~4 KiB per pnts tile heuristic
            pos + tiles
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_job_bytes_known_ops() {
        assert!(
            estimate_job_bytes(JobOp::LasParse {
                point_count: 1_000_000,
                has_color: true,
            }) > 0
        );
        assert!(
            estimate_job_bytes(JobOp::OctreeBuild {
                point_count: 1_000_000,
            }) > 0
        );
        assert!(
            estimate_job_bytes(JobOp::TilesetGenerate {
                point_count: 1_000_000,
                leaf_count: 10,
            }) > 0
        );
    }
}

// ===========================================================================
// WASM API
// ===========================================================================

#[wasm_bindgen(js_name = "ProcessingContext")]
pub struct WasmProcessingContext {
    inner: ProcessingContext,
}

impl Default for WasmProcessingContext {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = "ProcessingContext")]
impl WasmProcessingContext {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: ProcessingContext::new(),
        }
    }

    #[wasm_bindgen]
    pub fn reserve(&mut self, point_count: u32, has_color: bool) {
        self.inner.reserve(point_count as usize, has_color);
    }

    #[wasm_bindgen(getter, js_name = "reservedBytes")]
    pub fn reserved_bytes(&self) -> usize {
        self.inner.reserved_bytes()
    }

    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

/// Estimate job memory in bytes. Pass `op` as `"lasParse"`, `"octreeBuild"`, or `"tilesetGenerate"`.
#[wasm_bindgen(js_name = "estimateJobBytes")]
pub fn estimate_job_bytes_js(
    op: &str,
    point_count: u32,
    leaf_count: u32,
    has_color: bool,
) -> Result<usize, JsValue> {
    let job = match op {
        "lasParse" => JobOp::LasParse {
            point_count,
            has_color,
        },
        "octreeBuild" => JobOp::OctreeBuild { point_count },
        "tilesetGenerate" => JobOp::TilesetGenerate {
            point_count,
            leaf_count,
        },
        _ => {
            return Err(
                crate::errors::SpatialError::invalid_input(format!("unknown job op: {op}")).into(),
            )
        }
    };
    Ok(estimate_job_bytes(job))
}
