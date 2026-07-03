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
    LasParse {
        point_count: u32,
        has_color: bool,
    },
    OctreeBuild {
        point_count: u32,
    },
    TilesetGenerate {
        point_count: u32,
        leaf_count: u32,
    },
    /// Peak memory when tiles are emitted one-by-one (no full `TilesetResult` retention).
    TilesetIncremental {
        point_count: u32,
        max_leaf_points: u32,
        has_color: bool,
    },
    GeotiffParse {
        pixels: u32,
    },
    GeotiffTerrainTileset {
        pixels: u32,
    },
    CopcRegion {
        point_count: u32,
        has_color: bool,
    },
    /// LAS parse + octree + tileset (worst-case peak estimate).
    PointCloudPipeline {
        point_count: u32,
        leaf_count: u32,
        has_color: bool,
    },
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
        JobOp::OctreeBuild { point_count } => estimate_octree_build_bytes(point_count),
        JobOp::TilesetGenerate {
            point_count,
            leaf_count,
        } => estimate_tileset_generate_bytes(point_count, leaf_count),
        JobOp::TilesetIncremental {
            point_count,
            max_leaf_points,
            has_color,
        } => {
            let positions = point_count as usize * 3 * std::mem::size_of::<f32>();
            let colors = if has_color {
                point_count as usize * 3
            } else {
                0
            };
            let octree = estimate_octree_build_bytes(point_count) - positions;
            let largest_tile = max_leaf_points as usize * 14 + 4_096;
            positions + colors + octree + largest_tile
        }
        JobOp::GeotiffParse { pixels } => {
            let p = pixels as usize;
            p * std::mem::size_of::<f32>() + p / 4 + 65_536
        }
        JobOp::GeotiffTerrainTileset { pixels } => {
            let parse = estimate_job_bytes(JobOp::GeotiffParse { pixels });
            let pyramid = (pixels as usize) / 2 + 256 * 1024;
            parse + pyramid
        }
        JobOp::CopcRegion {
            point_count,
            has_color,
        } => estimate_job_bytes(JobOp::LasParse {
            point_count,
            has_color,
        }),
        JobOp::PointCloudPipeline {
            point_count,
            leaf_count,
            has_color,
        } => {
            let parse = estimate_job_bytes(JobOp::LasParse {
                point_count,
                has_color,
            });
            let octree = estimate_job_bytes(JobOp::OctreeBuild { point_count });
            let tiles = estimate_job_bytes(JobOp::TilesetGenerate {
                point_count,
                leaf_count,
            });
            parse.max(octree).max(tiles)
        }
    }
}

fn estimate_octree_build_bytes(point_count: u32) -> usize {
    let pos = point_count as usize * 3 * std::mem::size_of::<f32>();
    let nodes = (point_count as usize / 8_000 + 1) * 64;
    pos + nodes
}

fn estimate_tileset_generate_bytes(point_count: u32, leaf_count: u32) -> usize {
    let pos = point_count as usize * 3 * std::mem::size_of::<f32>();
    let tiles = leaf_count as usize * 4_096;
    pos + tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_job_bytes_known_ops() {
        // LasParse estimate is non-zero only when point-cloud provides the
        // real memory model; without the feature the estimate is intentionally 0.
        #[cfg(feature = "point-cloud")]
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
        assert!(
            estimate_job_bytes(JobOp::TilesetIncremental {
                point_count: 1_000_000,
                max_leaf_points: 50_000,
                has_color: false,
            }) < estimate_job_bytes(JobOp::TilesetGenerate {
                point_count: 1_000_000,
                leaf_count: 400,
            })
        );
        assert!(estimate_job_bytes(JobOp::GeotiffParse { pixels: 256 * 256 }) > 256 * 256 * 4);
        assert!(
            estimate_job_bytes(JobOp::PointCloudPipeline {
                point_count: 100_000,
                leaf_count: 20,
                has_color: false,
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

/// Estimate job memory in bytes.
///
/// `op`: `lasParse` | `octreeBuild` | `tilesetGenerate` | `tilesetIncremental` |
/// `geotiffParse` | `geotiffTerrainTileset` | `copcRegion` | `pointCloudPipeline`
///
/// `raster_width` / `raster_height` — pixel dimensions for GeoTIFF ops (else 0).
#[wasm_bindgen(js_name = "estimateJobBytes")]
pub fn estimate_job_bytes_js(
    op: &str,
    point_count: u32,
    leaf_count: u32,
    has_color: bool,
    raster_width: u32,
    raster_height: u32,
) -> Result<usize, JsValue> {
    let pixels = raster_width.saturating_mul(raster_height);
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
        "tilesetIncremental" => JobOp::TilesetIncremental {
            point_count,
            max_leaf_points: leaf_count.max(1),
            has_color,
        },
        "geotiffParse" => JobOp::GeotiffParse { pixels },
        "geotiffTerrainTileset" => JobOp::GeotiffTerrainTileset { pixels },
        "copcRegion" => JobOp::CopcRegion {
            point_count,
            has_color,
        },
        "pointCloudPipeline" => JobOp::PointCloudPipeline {
            point_count,
            leaf_count,
            has_color,
        },
        _ => {
            return Err(
                crate::errors::SpatialError::invalid_input(format!("unknown job op: {op}")).into(),
            )
        }
    };
    Ok(estimate_job_bytes(job))
}
