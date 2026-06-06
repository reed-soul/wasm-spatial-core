//! Spatial IR chunk export — glTF and 3D Tiles subset (Wave 2.5).

use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::pnts::{encode_pnts_tile, estimate_point_spacing};
use crate::spatial_ir::{Aabb, MeshChunk, PointCloudChunk};

/// Result of exporting a point cloud chunk to a single pnts tile + tileset.json.
#[derive(Debug, Clone)]
pub struct PointCloudTileExport {
    pub pnts: Vec<u8>,
    pub tileset_json: String,
    pub center: [f64; 3],
    pub bounds: [f64; 6],
}

impl PointCloudChunk {
    /// Export to one `.pnts` tile and a minimal `tileset.json` referencing it.
    pub fn export_to_pnts(
        &self,
        tile_uri: &str,
    ) -> Result<PointCloudTileExport, SpatialErrorDetail> {
        if self.vertex_count() == 0 {
            return Err(
                SpatialError::PointCloudError.with_detail("cannot export empty point cloud")
            );
        }

        let bounds = aabb_to_bounds(&self.metadata.aabb);
        let center = bounds_center(&bounds);
        let colors = rgb_colors_for_pnts(self.colors.as_deref(), self.vertex_count())?;

        let pnts = encode_pnts_tile(&self.positions, center, colors.as_deref())?;
        let spacing = estimate_point_spacing(&self.positions, None);
        let geometric_error = if spacing > 0.0 {
            spacing
        } else {
            bounds_diagonal(&bounds)
        };
        let tileset_json = build_minimal_tileset_json(&bounds, center, tile_uri, geometric_error);

        Ok(PointCloudTileExport {
            pnts,
            tileset_json,
            center,
            bounds,
        })
    }
}

impl MeshChunk {
    /// Export selected mesh as standalone GLB bytes (alias for `to_glb_bytes`).
    pub fn export_to_glb(&self) -> Vec<u8> {
        self.to_glb_bytes()
    }
}

fn aabb_to_bounds(aabb: &Aabb) -> [f64; 6] {
    [
        aabb.min[0],
        aabb.min[1],
        aabb.min[2],
        aabb.max[0],
        aabb.max[1],
        aabb.max[2],
    ]
}

fn bounds_center(bounds: &[f64; 6]) -> [f64; 3] {
    [
        (bounds[0] + bounds[3]) * 0.5,
        (bounds[1] + bounds[4]) * 0.5,
        (bounds[2] + bounds[5]) * 0.5,
    ]
}

fn bounds_diagonal(bounds: &[f64; 6]) -> f64 {
    let dx = bounds[3] - bounds[0];
    let dy = bounds[4] - bounds[1];
    let dz = bounds[5] - bounds[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn rgb_colors_for_pnts(
    colors: Option<&[u8]>,
    num_points: usize,
) -> Result<Option<Vec<u8>>, SpatialErrorDetail> {
    let Some(colors) = colors else {
        return Ok(None);
    };

    if colors.len() == num_points * 3 {
        return Ok(Some(colors.to_vec()));
    }

    if colors.len() == num_points * 4 {
        let rgb: Vec<u8> = colors
            .chunks_exact(4)
            .flat_map(|rgba| rgba[..3].iter().copied())
            .collect();
        return Ok(Some(rgb));
    }

    Err(SpatialError::PointCloudError.with_detail(format!(
        "color count mismatch: expected {} or {} bytes, got {}",
        num_points * 3,
        num_points * 4,
        colors.len()
    )))
}

/// Build a minimal single-tile 3D Tiles tileset.json.
pub fn build_minimal_tileset_json(
    bounds: &[f64; 6],
    center: [f64; 3],
    tile_uri: &str,
    geometric_error: f64,
) -> String {
    let hx = (bounds[3] - bounds[0]) * 0.5;
    let hy = (bounds[4] - bounds[1]) * 0.5;
    let hz = (bounds[5] - bounds[2]) * 0.5;

    format!(
        r#"{{"asset":{{"version":"1.0"}},"geometricError":{ge:.12},"root":{{"boundingVolume":{{"box":[{cx:.12},{cy:.12},{cz:.12},{hx:.12},0,0,0,{hy:.12},0,0,0,{hz:.12}]}},"geometricError":{ge:.12},"refine":"ADD","content":{{"uri":"{uri}"}}}}}}"#,
        ge = geometric_error,
        cx = center[0],
        cy = center[1],
        cz = center[2],
        hx = hx,
        hy = hy,
        hz = hz,
        uri = tile_uri,
    )
}

// ===========================================================================
// WASM API
// ===========================================================================

/// WASM result of exporting a point cloud to pnts + tileset.json.
#[wasm_bindgen]
pub struct WasmPointCloudTileExport {
    inner: PointCloudTileExport,
}

#[wasm_bindgen]
impl WasmPointCloudTileExport {
    #[wasm_bindgen(getter)]
    pub fn pnts(&self) -> js_sys::Uint8Array {
        js_sys::Uint8Array::from(&self.inner.pnts[..])
    }

    #[wasm_bindgen(getter, js_name = "tilesetJson")]
    pub fn tileset_json(&self) -> String {
        self.inner.tileset_json.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn center(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(&self.inner.center[..])
    }

    #[wasm_bindgen(getter)]
    pub fn bounds(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(&self.inner.bounds[..])
    }
}

/// Export point cloud positions to a single pnts tile + minimal tileset.json.
///
/// `positions`: flat `[x, y, z, ...]` Float32Array.
/// `colors`: optional RGB (`3×N`) or RGBA (`4×N`) bytes.
#[wasm_bindgen(js_name = "exportPointCloudToPnts")]
pub fn export_point_cloud_to_pnts(
    positions: &js_sys::Float32Array,
    colors: Option<Vec<u8>>,
    tile_uri: Option<String>,
) -> Result<WasmPointCloudTileExport, JsValue> {
    let mut pos_buf = vec![0.0f32; positions.length() as usize];
    positions.copy_to(&mut pos_buf);

    let mut chunk = PointCloudChunk {
        metadata: crate::spatial_ir::ChunkMeta::new("export"),
        positions: pos_buf,
        colors,
        normals: None,
    };
    chunk.refresh_metadata();

    let uri = tile_uri.unwrap_or_else(|| "tile_0.pnts".to_string());
    chunk
        .export_to_pnts(&uri)
        .map(|inner| WasmPointCloudTileExport { inner })
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_point_cloud_to_pnts() {
        let mut chunk = PointCloudChunk {
            metadata: crate::spatial_ir::ChunkMeta::new("las"),
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            colors: Some(vec![255, 0, 0, 0, 255, 0, 0, 0, 255]),
            normals: None,
        };
        chunk.refresh_metadata();

        let export = chunk.export_to_pnts("cloud.pnts").unwrap();
        assert!(export.pnts.len() >= 28);
        assert_eq!(&export.pnts[0..4], b"pnts");
        assert!(export.tileset_json.contains("cloud.pnts"));
        assert!(export.tileset_json.contains("boundingVolume"));
        assert!(export.tileset_json.contains("geometricError"));
    }

    #[test]
    fn test_export_rgba_colors() {
        let mut chunk = PointCloudChunk {
            metadata: crate::spatial_ir::ChunkMeta::new("las"),
            positions: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            colors: Some(vec![255, 0, 0, 255, 0, 255, 0, 255]),
            normals: None,
        };
        chunk.refresh_metadata();
        let export = chunk.export_to_pnts("t.pnts").unwrap();
        assert!(!export.pnts.is_empty());
    }

    #[test]
    fn test_mesh_export_to_glb() {
        let mut mesh = MeshChunk {
            metadata: crate::spatial_ir::ChunkMeta::new("glb"),
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
            indices: vec![0, 1, 2],
            normals: None,
            mode: MeshChunk::MODE_TRIANGLES,
        };
        mesh.refresh_metadata();
        let glb = mesh.export_to_glb();
        assert_eq!(&glb[0..4], b"glTF");
    }

    #[test]
    fn test_minimal_tileset_json_format() {
        let bounds = [0.0, 0.0, 0.0, 10.0, 10.0, 10.0];
        let json = build_minimal_tileset_json(&bounds, [5.0, 5.0, 5.0], "tile_0.pnts", 1.5);
        assert!(json.contains("\"refine\":\"ADD\""));
        assert!(json.contains("tile_0.pnts"));
    }
}
