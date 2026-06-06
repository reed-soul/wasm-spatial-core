//! Spatial IR — unified internal representation for ingest → edit → export.
//!
//! Wave 2 core: all spatial formats converge to [`SpatialChunk`] variants
//! before region selection, deformation, or tile export.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};

// ===========================================================================
// AABB
// ===========================================================================

/// Axis-aligned bounding box in the chunk's coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Default for Aabb {
    fn default() -> Self {
        Self {
            min: [f64::MAX, f64::MAX, f64::MAX],
            max: [f64::MIN, f64::MIN, f64::MIN],
        }
    }
}

impl Aabb {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.min[0] > self.max[0] || self.min[1] > self.max[1] || self.min[2] > self.max[2]
    }

    pub fn expand_point(&mut self, x: f64, y: f64, z: f64) {
        self.min[0] = self.min[0].min(x);
        self.min[1] = self.min[1].min(y);
        self.min[2] = self.min[2].min(z);
        self.max[0] = self.max[0].max(x);
        self.max[1] = self.max[1].max(y);
        self.max[2] = self.max[2].max(z);
    }

    pub fn from_positions(positions: &[f32]) -> Self {
        let mut aabb = Self::empty();
        for chunk in positions.chunks_exact(3) {
            aabb.expand_point(chunk[0] as f64, chunk[1] as f64, chunk[2] as f64);
        }
        aabb
    }

    pub fn contains_point(&self, x: f64, y: f64, z: f64) -> bool {
        x >= self.min[0]
            && x <= self.max[0]
            && y >= self.min[1]
            && y <= self.max[1]
            && z >= self.min[2]
            && z <= self.max[2]
    }

    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min[0] <= other.max[0]
            && self.max[0] >= other.min[0]
            && self.min[1] <= other.max[1]
            && self.max[1] >= other.min[1]
            && self.min[2] <= other.max[2]
            && self.max[2] >= other.min[2]
    }
}

// ===========================================================================
// Chunk metadata
// ===========================================================================

/// Metadata shared by all spatial chunk variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkMeta {
    /// CRS identifier (e.g. `"EPSG:4326"`) when known.
    pub crs: Option<String>,
    pub aabb: Aabb,
    /// Monotonic version — incremented on edit operations.
    pub version: u64,
    /// Source format label (e.g. `"glb"`, `"las"`, `"geotiff"`).
    pub source_format: Option<String>,
    /// Optional byte budget for downstream tile packing.
    pub byte_budget: Option<usize>,
}

impl ChunkMeta {
    pub fn new(source_format: impl Into<String>) -> Self {
        Self {
            crs: None,
            aabb: Aabb::empty(),
            version: 0,
            source_format: Some(source_format.into()),
            byte_budget: None,
        }
    }

    pub fn with_aabb(mut self, aabb: Aabb) -> Self {
        self.aabb = aabb;
        self
    }

    pub fn with_crs(mut self, crs: impl Into<String>) -> Self {
        self.crs = Some(crs.into());
        self
    }

    pub fn estimate_byte_size(&self) -> usize {
        self.byte_budget.unwrap_or(0)
    }

    pub fn bump_version(&mut self) {
        self.version = self.version.saturating_add(1);
    }
}

// ===========================================================================
// Chunk variants
// ===========================================================================

/// Point cloud spatial chunk.
#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudChunk {
    pub metadata: ChunkMeta,
    pub positions: Vec<f32>,
    pub colors: Option<Vec<u8>>,
    pub normals: Option<Vec<f32>>,
}

impl PointCloudChunk {
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    pub fn estimate_bytes(&self) -> usize {
        let mut size = self.positions.len() * std::mem::size_of::<f32>();
        if let Some(colors) = &self.colors {
            size += colors.len();
        }
        if let Some(normals) = &self.normals {
            size += normals.len() * std::mem::size_of::<f32>();
        }
        size
    }

    pub fn refresh_metadata(&mut self) {
        self.metadata.aabb = Aabb::from_positions(&self.positions);
        self.metadata.byte_budget = Some(self.estimate_bytes());
    }

    /// Select points whose position lies inside `region`.
    pub fn select_by_aabb(&self, region: &Aabb) -> Result<PointCloudChunk, SpatialErrorDetail> {
        if region.is_empty() {
            return Err(SpatialError::InvalidInput.with_detail("selection AABB is empty"));
        }

        let mut positions = Vec::new();
        let mut colors = self.colors.as_ref().map(|_| Vec::new());
        let mut normals = self.normals.as_ref().map(|_| Vec::new());

        for (i, chunk) in self.positions.chunks_exact(3).enumerate() {
            let x = chunk[0] as f64;
            let y = chunk[1] as f64;
            let z = chunk[2] as f64;
            if region.contains_point(x, y, z) {
                positions.extend_from_slice(chunk);
                if let (Some(src), Some(dst)) = (self.colors.as_ref(), colors.as_mut()) {
                    let base = i * 4;
                    if src.len() >= base + 4 {
                        dst.extend_from_slice(&src[base..base + 4]);
                    }
                }
                if let (Some(src), Some(dst)) = (self.normals.as_ref(), normals.as_mut()) {
                    let base = i * 3;
                    if src.len() >= base + 3 {
                        dst.extend_from_slice(&src[base..base + 3]);
                    }
                }
            }
        }

        if positions.is_empty() {
            return Err(SpatialError::GeometryError.with_detail("AABB selection is empty"));
        }

        let mut chunk = PointCloudChunk {
            metadata: self.metadata.clone(),
            positions,
            colors,
            normals,
        };
        chunk.metadata.bump_version();
        chunk.refresh_metadata();
        Ok(chunk)
    }
}

/// Indexed mesh spatial chunk (triangles or points).
#[derive(Debug, Clone, PartialEq)]
pub struct MeshChunk {
    pub metadata: ChunkMeta,
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Option<Vec<f32>>,
    /// glTF primitive mode: 0 = POINTS, 4 = TRIANGLES.
    pub mode: u32,
}

impl MeshChunk {
    pub const MODE_POINTS: u32 = 0;
    pub const MODE_TRIANGLES: u32 = 4;

    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    pub fn estimate_bytes(&self) -> usize {
        let mut size = self.positions.len() * std::mem::size_of::<f32>();
        size += self.indices.len() * std::mem::size_of::<u32>();
        if let Some(normals) = &self.normals {
            size += normals.len() * std::mem::size_of::<f32>();
        }
        size
    }

    pub fn refresh_metadata(&mut self) {
        self.metadata.aabb = Aabb::from_positions(&self.positions);
        self.metadata.byte_budget = Some(self.estimate_bytes());
    }

    fn vertex_in_region(&self, idx: u32, region: &Aabb) -> bool {
        let base = idx as usize * 3;
        if base + 2 >= self.positions.len() {
            return false;
        }
        region.contains_point(
            self.positions[base] as f64,
            self.positions[base + 1] as f64,
            self.positions[base + 2] as f64,
        )
    }

    /// Select triangles (or points) that intersect `region`.
    ///
    /// Triangles are kept when any vertex lies inside the AABB.
    pub fn select_by_aabb(&self, region: &Aabb) -> Result<MeshChunk, SpatialErrorDetail> {
        if region.is_empty() {
            return Err(SpatialError::InvalidInput.with_detail("selection AABB is empty"));
        }

        if self.mode == Self::MODE_POINTS || self.indices.is_empty() {
            return self.select_points_by_aabb(region);
        }

        let stride = 3;
        let mut kept_triangles: Vec<[u32; 3]> = Vec::new();
        for tri in self.indices.chunks_exact(stride) {
            let i0 = tri[0];
            let i1 = tri[1];
            let i2 = tri[2];
            if self.vertex_in_region(i0, region)
                || self.vertex_in_region(i1, region)
                || self.vertex_in_region(i2, region)
            {
                kept_triangles.push([i0, i1, i2]);
            }
        }

        if kept_triangles.is_empty() {
            return Err(SpatialError::GeometryError.with_detail("AABB selection is empty"));
        }

        let mut vertex_map: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        let mut new_positions = Vec::new();
        let mut new_normals = self.normals.as_ref().map(|_| Vec::new());
        let mut new_indices = Vec::with_capacity(kept_triangles.len() * 3);

        for [i0, i1, i2] in kept_triangles {
            for old_idx in [i0, i1, i2] {
                let new_idx = *vertex_map.entry(old_idx).or_insert_with(|| {
                    let ni = (new_positions.len() / 3) as u32;
                    let base = old_idx as usize * 3;
                    new_positions.extend_from_slice(&self.positions[base..base + 3]);
                    if let (Some(src), Some(dst)) = (self.normals.as_ref(), new_normals.as_mut()) {
                        if src.len() >= base + 3 {
                            dst.extend_from_slice(&src[base..base + 3]);
                        }
                    }
                    ni
                });
                new_indices.push(new_idx);
            }
        }

        let mut chunk = MeshChunk {
            metadata: self.metadata.clone(),
            positions: new_positions,
            indices: new_indices,
            normals: new_normals,
            mode: self.mode,
        };
        chunk.metadata.bump_version();
        chunk.refresh_metadata();
        Ok(chunk)
    }

    fn select_points_by_aabb(&self, region: &Aabb) -> Result<MeshChunk, SpatialErrorDetail> {
        let vertex_count = self.vertex_count();
        let mut new_positions = Vec::new();
        let mut new_normals = self.normals.as_ref().map(|_| Vec::new());

        for i in 0..vertex_count {
            let base = i * 3;
            if region.contains_point(
                self.positions[base] as f64,
                self.positions[base + 1] as f64,
                self.positions[base + 2] as f64,
            ) {
                new_positions.extend_from_slice(&self.positions[base..base + 3]);
                if let (Some(src), Some(dst)) = (self.normals.as_ref(), new_normals.as_mut()) {
                    dst.extend_from_slice(&src[base..base + 3]);
                }
            }
        }

        if new_positions.is_empty() {
            return Err(SpatialError::GeometryError.with_detail("AABB selection is empty"));
        }

        let mut chunk = MeshChunk {
            metadata: self.metadata.clone(),
            positions: new_positions,
            indices: Vec::new(),
            normals: new_normals,
            mode: Self::MODE_POINTS,
        };
        chunk.metadata.bump_version();
        chunk.refresh_metadata();
        Ok(chunk)
    }

    /// Export this chunk as GLB bytes using the existing writer.
    pub fn to_glb_bytes(&self) -> Vec<u8> {
        use crate::gltf_writer::{build_glb, MeshData};
        let mesh = MeshData {
            positions: self.positions.clone(),
            indices: self.indices.clone(),
            normals: self.normals.clone(),
            colors: None,
            material_index: None,
            mode: self.mode,
        };
        build_glb(&[mesh], &[])
    }
}

/// Heightfield (raster elevation) spatial chunk.
#[derive(Debug, Clone, PartialEq)]
pub struct HeightfieldChunk {
    pub metadata: ChunkMeta,
    pub heights: Vec<f32>,
    pub width: u32,
    pub height: u32,
    /// Geographic bounds: [west, south, east, north].
    pub bounds: [f64; 4],
}

impl HeightfieldChunk {
    pub fn estimate_bytes(&self) -> usize {
        self.heights.len() * std::mem::size_of::<f32>()
    }

    pub fn refresh_metadata(&mut self) {
        let west = self.bounds[0];
        let south = self.bounds[1];
        let east = self.bounds[2];
        let north = self.bounds[3];

        let mut min_h = f64::MAX;
        let mut max_h = f64::MIN;
        for &h in &self.heights {
            let hd = h as f64;
            min_h = min_h.min(hd);
            max_h = max_h.max(hd);
        }

        self.metadata.aabb = Aabb {
            min: [west, min_h, south],
            max: [east, max_h, north],
        };
        self.metadata.byte_budget = Some(self.estimate_bytes());
    }
}

/// Unified spatial chunk — all ingest paths converge here.
#[derive(Debug, Clone, PartialEq)]
pub enum SpatialChunk {
    PointCloud(PointCloudChunk),
    Mesh(MeshChunk),
    Heightfield(HeightfieldChunk),
}

impl SpatialChunk {
    pub fn metadata(&self) -> &ChunkMeta {
        match self {
            SpatialChunk::PointCloud(c) => &c.metadata,
            SpatialChunk::Mesh(c) => &c.metadata,
            SpatialChunk::Heightfield(c) => &c.metadata,
        }
    }

    pub fn estimate_bytes(&self) -> usize {
        match self {
            SpatialChunk::PointCloud(c) => c.estimate_bytes(),
            SpatialChunk::Mesh(c) => c.estimate_bytes(),
            SpatialChunk::Heightfield(c) => c.estimate_bytes(),
        }
    }
}

// ===========================================================================
// WASM API
// ===========================================================================

/// WASM-visible mesh chunk from Spatial IR.
#[wasm_bindgen]
pub struct WasmMeshChunk {
    inner: MeshChunk,
}

impl WasmMeshChunk {
    pub(crate) fn from_chunk(chunk: MeshChunk) -> Self {
        Self { inner: chunk }
    }

    pub fn inner(&self) -> &MeshChunk {
        &self.inner
    }
}

#[wasm_bindgen]
impl WasmMeshChunk {
    #[wasm_bindgen(getter)]
    pub fn positions(&self) -> js_sys::Float32Array {
        js_sys::Float32Array::from(&self.inner.positions[..])
    }

    #[wasm_bindgen(getter)]
    pub fn indices(&self) -> js_sys::Uint32Array {
        js_sys::Uint32Array::from(&self.inner.indices[..])
    }

    #[wasm_bindgen(getter)]
    pub fn normals(&self) -> js_sys::Float32Array {
        match &self.inner.normals {
            Some(n) => js_sys::Float32Array::from(&n[..]),
            None => js_sys::Float32Array::new_with_length(0),
        }
    }

    #[wasm_bindgen(js_name = "hasNormals")]
    pub fn has_normals(&self) -> bool {
        self.inner.normals.is_some()
    }

    #[wasm_bindgen(getter, js_name = "vertexCount")]
    pub fn vertex_count(&self) -> u32 {
        self.inner.vertex_count() as u32
    }

    #[wasm_bindgen(getter, js_name = "indexCount")]
    pub fn index_count(&self) -> u32 {
        self.inner.indices.len() as u32
    }

    #[wasm_bindgen(getter)]
    pub fn mode(&self) -> u32 {
        self.inner.mode
    }

    #[wasm_bindgen(getter, js_name = "version")]
    pub fn version(&self) -> u64 {
        self.inner.metadata.version
    }

    /// AABB min corner [x, y, z].
    #[wasm_bindgen(getter, js_name = "aabbMin")]
    pub fn aabb_min(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(&self.inner.metadata.aabb.min[..])
    }

    /// AABB max corner [x, y, z].
    #[wasm_bindgen(getter, js_name = "aabbMax")]
    pub fn aabb_max(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(&self.inner.metadata.aabb.max[..])
    }

    /// Export as GLB bytes.
    #[wasm_bindgen(js_name = "toGlb")]
    pub fn to_glb(&self) -> js_sys::Uint8Array {
        let bytes = self.inner.to_glb_bytes();
        let arr = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
        arr.copy_from(&bytes);
        arr
    }

    /// Select geometry inside an axis-aligned box.
    ///
    /// `min` and `max` are `[x, y, z]` corners in the chunk's coordinate frame.
    #[wasm_bindgen(js_name = "selectAabb")]
    pub fn select_aabb(
        &self,
        min: &js_sys::Float64Array,
        max: &js_sys::Float64Array,
    ) -> Result<WasmMeshChunk, JsValue> {
        if min.length() < 3 || max.length() < 3 {
            return Err(SpatialError::InvalidInput
                .with_detail("min and max must each have 3 components")
                .into());
        }

        let region = Aabb {
            min: [min.get_index(0), min.get_index(1), min.get_index(2)],
            max: [max.get_index(0), max.get_index(1), max.get_index(2)],
        };

        self.inner
            .select_by_aabb(&region)
            .map(WasmMeshChunk::from_chunk)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mesh() -> MeshChunk {
        let mut chunk = MeshChunk {
            metadata: ChunkMeta::new("test"),
            positions: vec![
                0.0, 0.0, 0.0, //
                1.0, 0.0, 0.0, //
                0.5, 1.0, 0.0, //
                5.0, 5.0, 5.0, //
                6.0, 5.0, 5.0, //
                5.5, 6.0, 5.0, //
            ],
            indices: vec![0, 1, 2, 3, 4, 5],
            normals: Some((0..6).flat_map(|_| [0.0, 1.0, 0.0]).collect()),
            mode: MeshChunk::MODE_TRIANGLES,
        };
        chunk.refresh_metadata();
        chunk
    }

    #[test]
    fn test_aabb_from_positions() {
        let aabb = Aabb::from_positions(&[0.0, 0.0, 0.0, 1.0, 2.0, 3.0]);
        assert_eq!(aabb.min, [0.0, 0.0, 0.0]);
        assert_eq!(aabb.max, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_chunk_meta_version_bump() {
        let mut meta = ChunkMeta::new("glb");
        assert_eq!(meta.version, 0);
        meta.bump_version();
        assert_eq!(meta.version, 1);
    }

    #[test]
    fn test_chunk_meta_json_roundtrip() {
        let meta = ChunkMeta::new("glb").with_crs("EPSG:4978").with_aabb(Aabb {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        });
        let json = serde_json::to_string(&meta).unwrap();
        let restored: ChunkMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.crs, meta.crs);
        assert_eq!(restored.aabb, meta.aabb);
    }

    #[test]
    fn test_spatial_chunk_variants() {
        let pc = SpatialChunk::PointCloud(PointCloudChunk {
            metadata: ChunkMeta::new("las"),
            positions: vec![0.0, 0.0, 0.0],
            colors: None,
            normals: None,
        });
        let mesh = SpatialChunk::Mesh(sample_mesh());
        let hf = SpatialChunk::Heightfield(HeightfieldChunk {
            metadata: ChunkMeta::new("geotiff"),
            heights: vec![10.0, 20.0],
            width: 2,
            height: 1,
            bounds: [0.0, 0.0, 1.0, 1.0],
        });

        assert_eq!(pc.metadata().source_format, Some("las".to_string()));
        assert!(mesh.estimate_bytes() > 0);
        assert_eq!(hf.metadata().source_format, Some("geotiff".to_string()));
    }

    #[test]
    fn test_mesh_select_by_aabb() {
        let mesh = sample_mesh();
        let region = Aabb {
            min: [-0.1, -0.1, -0.1],
            max: [2.0, 2.0, 2.0],
        };
        let selected = mesh.select_by_aabb(&region).unwrap();
        assert_eq!(selected.vertex_count(), 3);
        assert_eq!(selected.indices.len(), 3);
        assert_eq!(selected.metadata.version, 1);
    }

    #[test]
    fn test_mesh_select_empty_returns_error() {
        let mesh = sample_mesh();
        let region = Aabb {
            min: [100.0, 100.0, 100.0],
            max: [101.0, 101.0, 101.0],
        };
        let err = mesh.select_by_aabb(&region).unwrap_err();
        assert_eq!(err.code(), SpatialError::GeometryError.code());
    }

    #[test]
    fn test_point_cloud_select_by_aabb() {
        let pc = PointCloudChunk {
            metadata: ChunkMeta::new("las"),
            positions: vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0],
            colors: None,
            normals: None,
        };
        let region = Aabb {
            min: [-1.0, -1.0, -1.0],
            max: [1.0, 1.0, 1.0],
        };
        let selected = pc.select_by_aabb(&region).unwrap();
        assert_eq!(selected.vertex_count(), 1);
    }

    #[test]
    fn test_heightfield_metadata() {
        let mut hf = HeightfieldChunk {
            metadata: ChunkMeta::new("geotiff"),
            heights: vec![0.0, 5.0, 10.0, 15.0],
            width: 2,
            height: 2,
            bounds: [116.0, 39.0, 117.0, 40.0],
        };
        hf.refresh_metadata();
        assert_eq!(hf.metadata.aabb.min[1], 0.0);
        assert_eq!(hf.metadata.aabb.max[1], 15.0);
    }
}
