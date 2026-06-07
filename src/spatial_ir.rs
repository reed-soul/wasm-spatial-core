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
// Polygon extrusion (2D ring in XY + Z range)
// ===========================================================================

/// Vertical extrusion of a 2D polygon ring in the XY plane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolygonExtrusion {
    /// Closed ring `[x0, y0, x1, y1, …]` (first vertex need not repeat).
    pub ring: Vec<f64>,
    pub z_min: f64,
    pub z_max: f64,
}

impl PolygonExtrusion {
    pub fn new(ring: Vec<f64>, z_min: f64, z_max: f64) -> Self {
        Self { ring, z_min, z_max }
    }

    pub fn validate(&self) -> Result<(), SpatialErrorDetail> {
        if self.ring.len() < 6 || !self.ring.len().is_multiple_of(2) {
            return Err(SpatialError::InvalidInput
                .with_detail("polygon ring must have at least 3 vertices as [x, y, …]"));
        }
        if self.z_min > self.z_max {
            return Err(
                SpatialError::InvalidInput.with_detail("z_min must be <= z_max for extrusion")
            );
        }
        Ok(())
    }

    pub fn contains_point(&self, x: f64, y: f64, z: f64) -> bool {
        if z < self.z_min || z > self.z_max {
            return false;
        }
        point_in_ring_xy(x, y, &self.ring)
    }
}

/// Ray-casting point-in-polygon test for a single ring in the XY plane.
fn point_in_ring_xy(px: f64, py: f64, ring: &[f64]) -> bool {
    let n = ring.len() / 2;
    if n < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = n - 1;

    for i in 0..n {
        let xi = ring[i * 2];
        let yi = ring[i * 2 + 1];
        let xj = ring[j * 2];
        let yj = ring[j * 2 + 1];

        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }

    inside
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
        self.select_points(
            |x, y, z| region.contains_point(x, y, z),
            "AABB selection is empty",
        )
    }

    /// Select points inside a vertically extruded polygon (XY ring + Z range).
    pub fn select_by_polygon(
        &self,
        region: &PolygonExtrusion,
    ) -> Result<PointCloudChunk, SpatialErrorDetail> {
        region.validate()?;
        self.select_points(
            |x, y, z| region.contains_point(x, y, z),
            "polygon selection is empty",
        )
    }

    fn select_points<F>(
        &self,
        inside: F,
        empty_detail: &str,
    ) -> Result<PointCloudChunk, SpatialErrorDetail>
    where
        F: Fn(f64, f64, f64) -> bool,
    {
        let mut positions = Vec::new();
        let mut colors = self.colors.as_ref().map(|_| Vec::new());
        let mut normals = self.normals.as_ref().map(|_| Vec::new());

        for (i, chunk) in self.positions.chunks_exact(3).enumerate() {
            let x = chunk[0] as f64;
            let y = chunk[1] as f64;
            let z = chunk[2] as f64;
            if inside(x, y, z) {
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
            return Err(SpatialError::GeometryError.with_detail(empty_detail));
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
    /// Optional per-vertex UVs `[u0, v0, u1, v1, …]`.
    pub texcoords: Option<Vec<f32>>,
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
        if let Some(texcoords) = &self.texcoords {
            size += texcoords.len() * std::mem::size_of::<f32>();
        }
        size
    }

    pub fn refresh_metadata(&mut self) {
        self.metadata.aabb = Aabb::from_positions(&self.positions);
        self.metadata.byte_budget = Some(self.estimate_bytes());
    }

    fn vertex_position(&self, idx: u32) -> Option<(f64, f64, f64)> {
        let base = idx as usize * 3;
        if base + 2 >= self.positions.len() {
            return None;
        }
        Some((
            self.positions[base] as f64,
            self.positions[base + 1] as f64,
            self.positions[base + 2] as f64,
        ))
    }

    fn vertex_matches<F>(&self, idx: u32, inside: &F) -> bool
    where
        F: Fn(f64, f64, f64) -> bool,
    {
        self.vertex_position(idx)
            .is_some_and(|(x, y, z)| inside(x, y, z))
    }

    /// Build a mesh containing only the given triangles (vertex indices refer to `self`).
    pub(crate) fn build_subset(&self, triangles: &[[u32; 3]]) -> MeshChunk {
        let mut vertex_map: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        let mut new_positions = Vec::new();
        let mut new_normals = self.normals.as_ref().map(|_| Vec::new());
        let mut new_texcoords = self.texcoords.as_ref().map(|_| Vec::new());
        let mut new_indices = Vec::with_capacity(triangles.len() * 3);

        for [i0, i1, i2] in triangles {
            for old_idx in [*i0, *i1, *i2] {
                let new_idx = *vertex_map.entry(old_idx).or_insert_with(|| {
                    let ni = (new_positions.len() / 3) as u32;
                    let base = old_idx as usize * 3;
                    new_positions.extend_from_slice(&self.positions[base..base + 3]);
                    if let (Some(src), Some(dst)) = (self.normals.as_ref(), new_normals.as_mut()) {
                        if src.len() >= base + 3 {
                            dst.extend_from_slice(&src[base..base + 3]);
                        }
                    }
                    if let (Some(src), Some(dst)) =
                        (self.texcoords.as_ref(), new_texcoords.as_mut())
                    {
                        let tb = old_idx as usize * 2;
                        if src.len() >= tb + 2 {
                            dst.extend_from_slice(&src[tb..tb + 2]);
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
            texcoords: new_texcoords,
            mode: self.mode,
        };
        chunk.metadata.bump_version();
        chunk.refresh_metadata();
        chunk
    }

    /// Select triangles (or points) that intersect `region`.
    ///
    /// Triangles are kept when any vertex lies inside the AABB.
    pub fn select_by_aabb(&self, region: &Aabb) -> Result<MeshChunk, SpatialErrorDetail> {
        if region.is_empty() {
            return Err(SpatialError::InvalidInput.with_detail("selection AABB is empty"));
        }
        self.select_region(
            |x, y, z| region.contains_point(x, y, z),
            "AABB selection is empty",
        )
    }

    /// Select triangles (or points) inside a vertically extruded polygon.
    ///
    /// Triangles are kept when any vertex lies inside the extrusion.
    pub fn select_by_polygon(
        &self,
        region: &PolygonExtrusion,
    ) -> Result<MeshChunk, SpatialErrorDetail> {
        region.validate()?;
        self.select_region(
            |x, y, z| region.contains_point(x, y, z),
            "polygon selection is empty",
        )
    }

    fn select_region<F>(
        &self,
        inside: F,
        empty_detail: &str,
    ) -> Result<MeshChunk, SpatialErrorDetail>
    where
        F: Fn(f64, f64, f64) -> bool,
    {
        if self.mode == Self::MODE_POINTS || self.indices.is_empty() {
            return self.select_mesh_points(&inside, empty_detail);
        }

        let mut kept_triangles: Vec<[u32; 3]> = Vec::new();
        for tri in self.indices.chunks_exact(3) {
            let i0 = tri[0];
            let i1 = tri[1];
            let i2 = tri[2];
            if self.vertex_matches(i0, &inside)
                || self.vertex_matches(i1, &inside)
                || self.vertex_matches(i2, &inside)
            {
                kept_triangles.push([i0, i1, i2]);
            }
        }

        if kept_triangles.is_empty() {
            return Err(SpatialError::GeometryError.with_detail(empty_detail));
        }

        Ok(self.build_subset(&kept_triangles))
    }

    fn select_mesh_points<F>(
        &self,
        inside: &F,
        empty_detail: &str,
    ) -> Result<MeshChunk, SpatialErrorDetail>
    where
        F: Fn(f64, f64, f64) -> bool,
    {
        let vertex_count = self.vertex_count();
        let mut new_positions = Vec::new();
        let mut new_normals = self.normals.as_ref().map(|_| Vec::new());

        for i in 0..vertex_count {
            let base = i * 3;
            let x = self.positions[base] as f64;
            let y = self.positions[base + 1] as f64;
            let z = self.positions[base + 2] as f64;
            if inside(x, y, z) {
                new_positions.extend_from_slice(&self.positions[base..base + 3]);
                if let (Some(src), Some(dst)) = (self.normals.as_ref(), new_normals.as_mut()) {
                    dst.extend_from_slice(&src[base..base + 3]);
                }
            }
        }

        if new_positions.is_empty() {
            return Err(SpatialError::GeometryError.with_detail(empty_detail));
        }

        let mut chunk = MeshChunk {
            metadata: self.metadata.clone(),
            positions: new_positions,
            indices: Vec::new(),
            normals: new_normals,
            texcoords: None,
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

    #[wasm_bindgen(getter)]
    pub fn texcoords(&self) -> js_sys::Float32Array {
        match &self.inner.texcoords {
            Some(t) => js_sys::Float32Array::from(&t[..]),
            None => js_sys::Float32Array::new_with_length(0),
        }
    }

    #[wasm_bindgen(js_name = "hasNormals")]
    pub fn has_normals(&self) -> bool {
        self.inner.normals.is_some()
    }

    #[wasm_bindgen(js_name = "hasTexcoords")]
    pub fn has_texcoords(&self) -> bool {
        self.inner.texcoords.is_some()
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

    /// Select geometry inside a vertically extruded polygon (XY ring + Z range).
    ///
    /// `ring` is a flat `[x0, y0, x1, y1, …]` array with at least three vertices.
    #[wasm_bindgen(js_name = "selectPolygon")]
    pub fn select_polygon(
        &self,
        ring: &js_sys::Float64Array,
        z_min: f64,
        z_max: f64,
    ) -> Result<WasmMeshChunk, JsValue> {
        let len = ring.length() as usize;
        if len < 6 || !len.is_multiple_of(2) {
            return Err(SpatialError::InvalidInput
                .with_detail("ring must have at least 3 vertices as [x, y, …]")
                .into());
        }

        let mut ring_vec = vec![0.0f64; len];
        ring.copy_to(&mut ring_vec);

        let region = PolygonExtrusion::new(ring_vec, z_min, z_max);
        self.inner
            .select_by_polygon(&region)
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
            texcoords: None,
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
    fn test_mesh_select_by_polygon() {
        let mesh = sample_mesh();
        let region = PolygonExtrusion::new(vec![-0.1, -0.1, 2.0, -0.1, 2.0, 2.0], -1.0, 1.0);
        let selected = mesh.select_by_polygon(&region).unwrap();
        assert_eq!(selected.vertex_count(), 3);
        assert_eq!(selected.indices.len(), 3);
    }

    #[test]
    fn test_mesh_select_polygon_empty_returns_error() {
        let mesh = sample_mesh();
        let region =
            PolygonExtrusion::new(vec![100.0, 100.0, 101.0, 100.0, 101.0, 101.0], 0.0, 1.0);
        let err = mesh.select_by_polygon(&region).unwrap_err();
        assert_eq!(err.code(), SpatialError::GeometryError.code());
    }

    #[test]
    fn test_point_cloud_select_by_polygon() {
        let pc = PointCloudChunk {
            metadata: ChunkMeta::new("las"),
            positions: vec![0.5, 0.5, 0.0, 10.0, 10.0, 0.0],
            colors: None,
            normals: None,
        };
        let region = PolygonExtrusion::new(vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0], -1.0, 1.0);
        let selected = pc.select_by_polygon(&region).unwrap();
        assert_eq!(selected.vertex_count(), 1);
    }

    #[test]
    fn test_point_in_ring_xy_square() {
        let ring = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        assert!(point_in_ring_xy(0.5, 0.5, &ring));
        assert!(!point_in_ring_xy(1.5, 0.5, &ring));
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
