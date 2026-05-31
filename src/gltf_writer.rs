//! glTF / GLB Writer
//!
//! Build glTF 2.0 scenes programmatically in WASM and export as
//! binary GLB or standalone JSON + BIN. Geometry is consumed as
//! flat typed arrays directly from JS with zero-copy transfer.
//!
//! Provides:
//! - `GltfBuilder` — low-level builder for multi-mesh scenes
//! - `pointCloudToGlb()` — one-shot point cloud → GLB (POINTS mode)
//! - `terrainToGlb()` — elevation grid → GLB mesh (TRIANGLES mode)
//! - `meshToGlb()` — generic indexed mesh → GLB

use serde::Serialize;
use wasm_bindgen::prelude::*;

// ===========================================================================
// glTF JSON model (minimal subset needed for our use case)
// ===========================================================================

#[derive(Serialize)]
struct GltfScene {
    nodes: Vec<u32>,
}

#[derive(Serialize)]
struct GltfNode {
    mesh: u32,
}

#[derive(Serialize)]
struct GltfMesh {
    primitives: Vec<GltfPrimitive>,
}

#[derive(Serialize)]
struct GltfPrimitive {
    attributes: serde_json::Map<String, serde_json::Value>,
    mode: Option<u32>, // 0 = POINTS, 4 = TRIANGLES, 5 = TRIANGLE_STRIP
    material: Option<u32>,
}

#[derive(Serialize)]
struct GltfAccessor {
    #[serde(rename = "bufferView")]
    buffer_view: u32,
    #[serde(rename = "byteOffset")]
    byte_offset: u32,
    #[serde(rename = "componentType")]
    component_type: u32,
    count: u32,
    #[serde(rename = "type")]
    accessor_type: String,
    #[serde(rename = "min")]
    min: Option<Vec<f64>>,
    #[serde(rename = "max")]
    max: Option<Vec<f64>>,
}

#[derive(Serialize)]
struct GltfBufferView {
    buffer: u32,
    #[serde(rename = "byteOffset")]
    byte_offset: u32,
    #[serde(rename = "byteLength")]
    byte_length: u32,
    #[serde(rename = "target")]
    target: Option<u32>, // 34962 = ARRAY_BUFFER, 34963 = ELEMENT_ARRAY_BUFFER
}

#[derive(Serialize)]
struct GltfBuffer {
    #[serde(rename = "byteLength")]
    byte_length: u32,
}

#[derive(Clone, Serialize)]
pub(crate) struct GltfMaterial {
    #[serde(rename = "pbrMetallicRoughness")]
    pbr_metallic_roughness: GltfPbr,
    #[serde(rename = "doubleSided")]
    double_sided: Option<bool>,
    #[serde(rename = "doubleSided")]
    #[allow(dead_code)]
    alpha_mode: Option<String>,
}

#[derive(Serialize, Clone)]
struct GltfPbr {
    #[serde(rename = "baseColorFactor")]
    base_color_factor: [f32; 4],
    metallic_factor: f32,
    roughness_factor: f32,
}

#[derive(Serialize)]
struct GltfRoot {
    asset: GltfAsset,
    scene: Option<u32>,
    scenes: Vec<GltfScene>,
    nodes: Vec<GltfNode>,
    meshes: Vec<GltfMesh>,
    accessors: Vec<GltfAccessor>,
    #[serde(rename = "bufferViews")]
    buffer_views: Vec<GltfBufferView>,
    buffers: Vec<GltfBuffer>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    materials: Vec<GltfMaterial>,
}

#[derive(Serialize)]
struct GltfAsset {
    version: String,
    generator: String,
}

// ===========================================================================
// Builder
// ===========================================================================

pub(crate) struct MeshData {
    pub(crate) positions: Vec<f32>,
    pub(crate) indices: Vec<u32>,
    pub(crate) normals: Option<Vec<f32>>,
    pub(crate) colors: Option<Vec<u8>>,
    pub(crate) material_index: Option<usize>,
    pub(crate) mode: u32, // 4 = TRIANGLES, 0 = POINTS
}

/// glTF 2.0 builder — collect meshes and materials, then export as GLB or JSON.
#[wasm_bindgen]
pub struct GltfBuilder {
    meshes: Vec<MeshData>,
    materials: Vec<GltfMaterial>,
}

impl Default for GltfBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl GltfBuilder {
    /// Create a new empty glTF builder.
    #[wasm_bindgen(constructor)]
    pub fn new() -> GltfBuilder {
        GltfBuilder {
            meshes: Vec::new(),
            materials: Vec::new(),
        }
    }

    /// Add a mesh with positions, indices, and optional normals.
    ///
    /// - `positions`: Flat `Float32Array` `[x0, y0, z0, x1, y1, z1, ...]`
    /// - `indices`: Flat `Uint32Array` `[i0, i1, i2, ...]`
    /// - `normals`: Optional flat `Float32Array` `[nx0, ny0, nz0, ...]` (may be `null`)
    /// - `material_index`: Material index (0-based), or `-1` for no material.
    #[wasm_bindgen(js_name = "addMesh")]
    pub fn add_mesh(
        &mut self,
        positions: &js_sys::Float32Array,
        indices: &js_sys::Uint32Array,
        normals: &js_sys::Float32Array,
        material_index: i32,
    ) {
        let mut pos_buf = vec![0.0f32; positions.length() as usize];
        positions.copy_to(&mut pos_buf);

        let mut idx_buf = vec![0u32; indices.length() as usize];
        indices.copy_to(&mut idx_buf);

        let has_normals = normals.length() > 0;
        let mut norm_buf = vec![0.0f32; normals.length() as usize];
        if has_normals {
            normals.copy_to(&mut norm_buf);
        }

        self.meshes.push(MeshData {
            positions: pos_buf,
            indices: idx_buf,
            normals: if has_normals { Some(norm_buf) } else { None },
            colors: None,
            material_index: if material_index >= 0 {
                Some(material_index as usize)
            } else {
                None
            },
            mode: 4, // TRIANGLES
        });
    }

    /// Add a material with base color (RGBA 0–1 range).
    #[wasm_bindgen(js_name = "addMaterial")]
    pub fn add_material(&mut self, r: f32, g: f32, b: f32, a: f32) -> u32 {
        let idx = self.materials.len() as u32;
        self.materials.push(GltfMaterial {
            pbr_metallic_roughness: GltfPbr {
                base_color_factor: [r, g, b, a],
                metallic_factor: 0.0,
                roughness_factor: 1.0,
            },
            double_sided: Some(true),
            alpha_mode: None,
        });
        idx
    }

    /// Export as binary GLB (`Uint8Array`).
    #[wasm_bindgen(js_name = "toGlb")]
    pub fn to_glb(&self) -> js_sys::Uint8Array {
        let bytes = build_glb(&self.meshes, &self.materials);
        let arr = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
        arr.copy_from(&bytes);
        arr
    }

    /// Export as glTF JSON string (no binary — positions/indices as base64).
    #[wasm_bindgen(js_name = "toGltfJson")]
    pub fn to_gltf_json(&self) -> String {
        let (json_bytes, _) = build_glb_parts(&self.meshes, &self.materials);
        String::from_utf8_lossy(&json_bytes).to_string()
    }
}

// ===========================================================================
// One-shot convenience functions (no builder needed)
// ===========================================================================

/// Convert a point cloud directly to a GLB file (POINTS primitive mode).
///
/// # Arguments
/// - `positions`: `Float32Array` `[x0, y0, z0, x1, y1, z1, ...]`
/// - `colors`: Optional `Uint8Array` `[r0, g0, b0, a0, r1, ...]` (RGBA per vertex)
/// - `normals`: Optional `Float32Array` `[nx0, ny0, nz0, ...]`
///
/// # Returns
/// `Uint8Array` containing the complete GLB binary.
#[wasm_bindgen(js_name = "pointCloudToGlb")]
pub fn point_cloud_to_glb(
    positions: &js_sys::Float32Array,
    colors: Option<Vec<u8>>,
    normals: Option<js_sys::Float32Array>,
) -> js_sys::Uint8Array {
    let mut pos_buf = vec![0.0f32; positions.length() as usize];
    positions.copy_to(&mut pos_buf);

    let color_buf = colors.filter(|c| !c.is_empty());

    let mut norm_buf_opt: Option<Vec<f32>> = None;
    if let Some(ref normals_arr) = normals {
        let len = normals_arr.length() as usize;
        if len > 0 {
            let mut buf = vec![0.0f32; len];
            normals_arr.copy_to(&mut buf);
            norm_buf_opt = Some(buf);
        }
    }

    // Build sequential indices for POINTS mode (not actually used but glTF requires
    // us to have no indices for POINTS; however we set mode=0 so indices are optional)
    let mesh = MeshData {
        positions: pos_buf,
        indices: Vec::new(), // POINTS mode doesn't use indices
        normals: norm_buf_opt,
        colors: color_buf,
        material_index: None,
        mode: 0, // POINTS
    };

    let bytes = build_glb(&[mesh], &[]);
    let arr = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
    arr.copy_from(&bytes);
    arr
}

/// Convert a terrain heightmap directly to a GLB mesh (TRIANGLES primitive mode).
///
/// Automatically generates normals from the height gradient.
///
/// # Arguments
/// - `heights`: `Float32Array` of elevation values (row-major, bottom-to-top or top-to-bottom)
/// - `width`: Number of columns in the grid
/// - `height`: Number of rows in the grid
/// - `bounds`: `[west, south, east, north]` in geographic or projected coordinates
///
/// # Returns
/// `Uint8Array` containing the complete GLB binary.
#[wasm_bindgen(js_name = "terrainToGlb")]
pub fn terrain_to_glb(
    heights: &js_sys::Float32Array,
    width: u32,
    height: u32,
    bounds: &js_sys::Float64Array,
) -> js_sys::Uint8Array {
    let mut h_buf = vec![0.0f32; heights.length() as usize];
    heights.copy_to(&mut h_buf);

    let mut b_buf = vec![0.0f64; bounds.length() as usize];
    bounds.copy_to(&mut b_buf);

    let (positions, indices, normals) =
        terrain_to_mesh(&h_buf, width as usize, height as usize, &b_buf);

    let mesh = MeshData {
        positions,
        indices,
        normals: Some(normals),
        colors: None,
        material_index: None,
        mode: 4, // TRIANGLES
    };

    let bytes = build_glb(&[mesh], &[]);
    let arr = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
    arr.copy_from(&bytes);
    arr
}

/// Convert a generic indexed mesh directly to a GLB file (TRIANGLES primitive mode).
///
/// # Arguments
/// - `vertices`: `Float32Array` `[x0, y0, z0, x1, y1, z1, ...]`
/// - `indices`: `Uint32Array` `[i0, i1, i2, ...]`
/// - `normals`: Optional `Float32Array` `[nx0, ny0, nz0, ...]`
///
/// # Returns
/// `Uint8Array` containing the complete GLB binary.
#[wasm_bindgen(js_name = "meshToGlb")]
pub fn mesh_to_glb(
    vertices: &js_sys::Float32Array,
    indices: &js_sys::Uint32Array,
    normals: Option<js_sys::Float32Array>,
) -> js_sys::Uint8Array {
    let mut v_buf = vec![0.0f32; vertices.length() as usize];
    vertices.copy_to(&mut v_buf);

    let mut i_buf = vec![0u32; indices.length() as usize];
    indices.copy_to(&mut i_buf);

    let mut norm_buf_opt: Option<Vec<f32>> = None;
    if let Some(ref normals_arr) = normals {
        let len = normals_arr.length() as usize;
        if len > 0 {
            let mut buf = vec![0.0f32; len];
            normals_arr.copy_to(&mut buf);
            norm_buf_opt = Some(buf);
        }
    }

    let mesh = MeshData {
        positions: v_buf,
        indices: i_buf,
        normals: norm_buf_opt,
        colors: None,
        material_index: None,
        mode: 4, // TRIANGLES
    };

    let bytes = build_glb(&[mesh], &[]);
    let arr = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
    arr.copy_from(&bytes);
    arr
}

// ===========================================================================
// Core GLB building (pure Rust, no WASM dependency — testable)
// ===========================================================================

pub(crate) fn build_glb(meshes: &[MeshData], materials: &[GltfMaterial]) -> Vec<u8> {
    let (json_bytes, bin_bytes) = build_glb_parts(meshes, materials);
    let total_length = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();

    let mut glb = Vec::with_capacity(total_length);

    // GLB header: magic(4) + version(4) + totalLength(4)
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2u32.to_le_bytes()); // version 2
    glb.extend_from_slice(&(total_length as u32).to_le_bytes());

    // JSON chunk: length(4) + type(4) + data
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(b"JSON");
    glb.extend_from_slice(&json_bytes);

    // BIN chunk: length(4) + type(4) + data
    glb.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(b"BIN\0");
    glb.extend_from_slice(&bin_bytes);

    glb
}

fn build_glb_parts(meshes: &[MeshData], materials: &[GltfMaterial]) -> (Vec<u8>, Vec<u8>) {
    let mut accessors = Vec::new();
    let mut buffer_views = Vec::new();
    let mut gltf_meshes = Vec::new();
    let mut gltf_nodes = Vec::new();
    let mut bin_data: Vec<u8> = Vec::new();
    let mut bin_offset: u32 = 0;

    for (mesh_idx, mesh) in meshes.iter().enumerate() {
        let vertex_count = mesh.positions.len() as u32 / 3;
        let mut primitives = Vec::new();

        // ── Position accessor + buffer view ──────────────────
        let pos_bytes = mesh.positions.len() as u32 * 4;
        let pos_bv_idx = buffer_views.len() as u32;
        buffer_views.push(GltfBufferView {
            buffer: 0,
            byte_offset: bin_offset,
            byte_length: pos_bytes,
            target: Some(34962), // ARRAY_BUFFER
        });

        // Compute min/max
        let mut min_pos = vec![f64::MAX, f64::MAX, f64::MAX];
        let mut max_pos = vec![f64::MIN, f64::MIN, f64::MIN];
        for i in 0..mesh.positions.len() / 3 {
            for j in 0..3 {
                let v = mesh.positions[i * 3 + j] as f64;
                min_pos[j] = min_pos[j].min(v);
                max_pos[j] = max_pos[j].max(v);
            }
        }

        let pos_acc_idx = accessors.len() as u32;
        accessors.push(GltfAccessor {
            buffer_view: pos_bv_idx,
            byte_offset: 0,
            component_type: 5126, // FLOAT
            count: vertex_count,
            accessor_type: "VEC3".to_string(),
            min: Some(min_pos),
            max: Some(max_pos),
        });

        bin_data.extend_from_slice(cast_slice_f32(&mesh.positions));
        bin_offset += pos_bytes;

        // ── Normal accessor (if present) ───────────────────────
        if let Some(ref normals) = mesh.normals {
            let norm_bytes = normals.len() as u32 * 4;
            let norm_bv_idx = buffer_views.len() as u32;
            buffer_views.push(GltfBufferView {
                buffer: 0,
                byte_offset: bin_offset,
                byte_length: norm_bytes,
                target: Some(34962),
            });

            accessors.push(GltfAccessor {
                buffer_view: norm_bv_idx,
                byte_offset: 0,
                component_type: 5126,
                count: vertex_count,
                accessor_type: "VEC3".to_string(),
                min: None,
                max: None,
            });

            bin_data.extend_from_slice(cast_slice_f32(normals));
            bin_offset += norm_bytes;
        }

        // ── Color accessor (if present) ──────────────────────
        if let Some(ref colors) = mesh.colors {
            let color_bytes = colors.len() as u32;
            let color_bv_idx = buffer_views.len() as u32;
            buffer_views.push(GltfBufferView {
                buffer: 0,
                byte_offset: bin_offset,
                byte_length: color_bytes,
                target: Some(34962),
            });

            accessors.push(GltfAccessor {
                buffer_view: color_bv_idx,
                byte_offset: 0,
                component_type: 5121, // UNSIGNED_BYTE
                count: vertex_count,
                accessor_type: "VEC4".to_string(),
                min: None,
                max: None,
            });

            bin_data.extend_from_slice(colors);
            bin_offset += color_bytes;
        }

        // ── Index accessor + buffer view ──────────────────────
        if !mesh.indices.is_empty() {
            let idx_bytes = mesh.indices.len() as u32 * 4;
            let idx_bv_idx = buffer_views.len() as u32;
            buffer_views.push(GltfBufferView {
                buffer: 0,
                byte_offset: bin_offset,
                byte_length: idx_bytes,
                target: Some(34963), // ELEMENT_ARRAY_BUFFER
            });

            accessors.push(GltfAccessor {
                buffer_view: idx_bv_idx,
                byte_offset: 0,
                component_type: 5125, // UNSIGNED_INT
                count: mesh.indices.len() as u32,
                accessor_type: "SCALAR".to_string(),
                min: None,
                max: None,
            });

            bin_data.extend_from_slice(cast_slice_u32(&mesh.indices));
            bin_offset += idx_bytes;
        }

        // ── Build primitive attributes map ─────────────────────
        let mut attrs = serde_json::Map::new();
        attrs.insert("POSITION".to_string(), serde_json::json!(pos_acc_idx));

        let mut acc_idx = pos_acc_idx + 1;
        if mesh.normals.is_some() {
            attrs.insert("NORMAL".to_string(), serde_json::json!(acc_idx));
            acc_idx += 1;
        }
        if mesh.colors.is_some() {
            attrs.insert("COLOR_0".to_string(), serde_json::json!(acc_idx));
            // acc_idx += 1; // no more attributes below
        }

        primitives.push(GltfPrimitive {
            attributes: attrs,
            mode: Some(mesh.mode),
            material: mesh.material_index.map(|m| m as u32),
        });

        gltf_meshes.push(GltfMesh { primitives });
        gltf_nodes.push(GltfNode {
            mesh: mesh_idx as u32,
        });
    }

    // Pad BIN data to 4-byte alignment
    while !bin_data.len().is_multiple_of(4) {
        bin_data.push(0x20); // space padding
    }

    let root = GltfRoot {
        asset: GltfAsset {
            version: "2.0".to_string(),
            generator: "wasm-spatial-core".to_string(),
        },
        scene: if !gltf_nodes.is_empty() {
            Some(0)
        } else {
            None
        },
        scenes: vec![GltfScene {
            nodes: (0..gltf_nodes.len() as u32).collect(),
        }],
        nodes: gltf_nodes,
        meshes: gltf_meshes,
        accessors,
        buffer_views,
        buffers: vec![GltfBuffer {
            byte_length: bin_data.len() as u32,
        }],
        materials: materials.to_vec(),
    };

    let json_str = serde_json::to_string(&root).unwrap_or_default();
    let mut json_bytes = json_str.into_bytes();

    // Pad JSON to 4-byte alignment with spaces (0x20)
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(0x20);
    }

    (json_bytes, bin_data)
}

// ===========================================================================
// Terrain grid → mesh conversion
// ===========================================================================

/// Convert a heightmap to triangle mesh data with auto-generated normals.
///
/// `bounds` is `[west, south, east, north]`.
/// `heights` is row-major, row 0 = south (bottom).
fn terrain_to_mesh(
    heights: &[f32],
    width: usize,
    height: usize,
    bounds: &[f64],
) -> (Vec<f32>, Vec<u32>, Vec<f32>) {
    assert_eq!(heights.len(), width * height, "heights length mismatch");

    let west = if !bounds.is_empty() { bounds[0] } else { 0.0 };
    let south = if bounds.len() > 1 { bounds[1] } else { 0.0 };
    let east = if bounds.len() > 2 { bounds[2] } else { 1.0 };
    let north = if bounds.len() > 3 { bounds[3] } else { 1.0 };

    let dx = if width > 1 {
        (east - west) / (width - 1) as f64
    } else {
        0.0
    };
    let dy = if height > 1 {
        (north - south) / (height - 1) as f64
    } else {
        0.0
    };

    let n_verts = width * height;
    let mut positions = Vec::with_capacity(n_verts * 3);
    let mut normals = vec![0.0f32; n_verts * 3];

    // Generate positions
    for row in 0..height {
        for col in 0..width {
            let x = west + col as f64 * dx;
            let z = south + row as f64 * dy;
            let y = heights[row * width + col] as f64;
            positions.push(x as f32);
            positions.push(y as f32);
            positions.push(z as f32);
        }
    }

    // Generate normals via cross product of adjacent edges
    for row in 0..height {
        for col in 0..width {
            let idx = row * width + col;
            let h = heights[idx];

            // Finite differences for gradient
            let h_left = if col > 0 { heights[idx - 1] } else { h };
            let h_right = if col < width - 1 { heights[idx + 1] } else { h };
            let h_down = if row > 0 { heights[idx - width] } else { h };
            let h_up = if row < height - 1 {
                heights[idx + width]
            } else {
                h
            };

            // Normal = normalize(cross(tangent_x, tangent_z))
            // tangent_x = (2*dx, h_right - h_left, 0)
            // tangent_z = (0, h_up - h_down, 2*dy)
            let gx = h_right - h_left;
            let gz = h_up - h_down;

            let nx = -gx;
            let scale = (dx * dy * (width as f64) * (height as f64)).max(1.0) as f32;
            let ny = 2.0 * scale;
            let nz = -gz;

            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            if len > 1e-10 {
                let inv_len = 1.0 / len;
                normals[idx * 3] = nx * inv_len;
                normals[idx * 3 + 1] = ny * inv_len;
                normals[idx * 3 + 2] = nz * inv_len;
            } else {
                // Flat surface → normal points up
                normals[idx * 3] = 0.0;
                normals[idx * 3 + 1] = 1.0;
                normals[idx * 3 + 2] = 0.0;
            }
        }
    }

    // Generate indices (two triangles per grid cell)
    let n_cells = (width - 1) * (height - 1);
    let mut indices = Vec::with_capacity(n_cells * 6);
    for row in 0..(height - 1) {
        for col in 0..(width - 1) {
            let tl = row * width + col;
            let tr = tl + 1;
            let bl = tl + width;
            let br = bl + 1;
            indices.push(tl as u32);
            indices.push(bl as u32);
            indices.push(tr as u32);
            indices.push(tr as u32);
            indices.push(bl as u32);
            indices.push(br as u32);
        }
    }

    (positions, indices, normals)
}

// ===========================================================================
// Safe byte casting
// ===========================================================================

fn cast_slice_f32(slice: &[f32]) -> &[u8] {
    // SAFETY: f32 is Copy with no padding; byte repr is well-defined.
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}

fn cast_slice_u32(slice: &[u32]) -> &[u8] {
    // SAFETY: u32 is Copy with no padding; byte repr is well-defined.
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: add a mesh directly to the builder (bypasses js_sys typed arrays).
    fn add_test_mesh(
        builder: &mut GltfBuilder,
        positions: Vec<f32>,
        indices: Vec<u32>,
        normals: Option<Vec<f32>>,
        material_index: i32,
    ) {
        builder.meshes.push(MeshData {
            positions,
            indices,
            normals,
            colors: None,
            material_index: if material_index >= 0 {
                Some(material_index as usize)
            } else {
                None
            },
            mode: 4, // TRIANGLES
        });
    }

    // -------------------------------------------------------------------------
    // Builder tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_gltf_builder_new() {
        let builder = GltfBuilder::new();
        assert_eq!(builder.meshes.len(), 0);
        assert_eq!(builder.materials.len(), 0);
    }

    #[test]
    fn test_gltf_builder_add_material() {
        let mut builder = GltfBuilder::new();
        let idx = builder.add_material(1.0, 0.0, 0.0, 1.0);
        assert_eq!(idx, 0);
        assert_eq!(builder.materials.len(), 1);
    }

    #[test]
    fn test_gltf_builder_glb_magic() {
        let mut builder = GltfBuilder::new();
        add_test_mesh(
            &mut builder,
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
            vec![0, 1, 2],
            None,
            -1,
        );

        let (json_bytes, bin_bytes) = build_glb_parts(&builder.meshes, &builder.materials);
        let glb = assemble_glb(&json_bytes, &bin_bytes);

        assert!(glb.len() >= 12, "GLB too small: {} bytes", glb.len());
        assert_eq!(&glb[0..4], b"glTF", "GLB magic mismatch");

        let version = u32::from_le_bytes([glb[4], glb[5], glb[6], glb[7]]);
        assert_eq!(version, 2, "GLB version should be 2");

        let total_len = u32::from_le_bytes([glb[8], glb[9], glb[10], glb[11]]);
        assert_eq!(total_len as usize, glb.len(), "GLB total length mismatch");
    }

    #[test]
    fn test_gltf_builder_glb_json_chunk() {
        let mut builder = GltfBuilder::new();
        add_test_mesh(
            &mut builder,
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            vec![0, 1],
            None,
            -1,
        );

        let (json_bytes, bin_bytes) = build_glb_parts(&builder.meshes, &builder.materials);
        let glb = assemble_glb(&json_bytes, &bin_bytes);

        // JSON chunk starts at offset 12
        let json_chunk_type = &glb[16..20];
        assert_eq!(json_chunk_type, b"JSON", "First chunk type should be JSON");

        let json_len = u32::from_le_bytes([glb[12], glb[13], glb[14], glb[15]]) as usize;
        let bin_chunk_start = 12 + 8 + json_len;
        let bin_chunk_type = &glb[bin_chunk_start + 4..bin_chunk_start + 8];
        assert_eq!(bin_chunk_type, b"BIN\0", "Second chunk type should be BIN");
    }

    #[test]
    fn test_gltf_builder_to_json() {
        let mut builder = GltfBuilder::new();
        add_test_mesh(
            &mut builder,
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            vec![0, 1],
            None,
            -1,
        );

        let (json_bytes, _) = build_glb_parts(&builder.meshes, &builder.materials);
        let json = String::from_utf8_lossy(&json_bytes);

        assert!(json.contains("\"asset\""), "JSON should contain asset");
        assert!(
            json.contains("\"version\":\"2.0\""),
            "JSON should specify glTF 2.0"
        );
        assert!(json.contains("\"meshes\""), "JSON should contain meshes");
    }

    #[test]
    fn test_gltf_builder_with_material() {
        let mut builder = GltfBuilder::new();
        builder.add_material(0.8, 0.2, 0.1, 1.0);
        add_test_mesh(
            &mut builder,
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
            vec![0, 1, 2],
            None,
            0,
        );

        let glb = build_glb(&builder.meshes, &builder.materials);
        assert!(!glb.is_empty());
        assert_eq!(&glb[0..4], b"glTF");

        let (json_bytes, _) = build_glb_parts(&builder.meshes, &builder.materials);
        let json = String::from_utf8_lossy(&json_bytes);
        assert!(
            json.contains("\"materials\""),
            "JSON should contain materials"
        );
    }

    // -------------------------------------------------------------------------
    // Point cloud one-shot test
    // -------------------------------------------------------------------------

    #[test]
    fn test_point_cloud_to_glb() {
        let positions = vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0];
        let mesh = MeshData {
            positions: positions.clone(),
            indices: Vec::new(),
            normals: None,
            colors: None,
            material_index: None,
            mode: 0, // POINTS
        };

        let glb = build_glb(&[mesh], &[]);
        assert!(glb.len() >= 12);
        assert_eq!(&glb[0..4], b"glTF");

        let (json_bytes, _) = build_glb_parts(
            &[MeshData {
                positions,
                indices: Vec::new(),
                normals: None,
                colors: None,
                material_index: None,
                mode: 0,
            }],
            &[],
        );
        let json = String::from_utf8_lossy(&json_bytes);
        // POINTS mode should not have indices
        assert!(json.contains("\"mode\":0"), "Should be POINTS mode (0)");
    }

    #[test]
    fn test_point_cloud_with_colors() {
        let positions = vec![0.0f32, 0.0, 0.0, 1.0, 1.0, 1.0];
        let colors = vec![255u8, 0, 0, 255, 0, 255, 0, 255]; // RGBA per vertex
        let mesh = MeshData {
            positions,
            indices: Vec::new(),
            normals: None,
            colors: Some(colors),
            material_index: None,
            mode: 0,
        };

        let glb = build_glb(&[mesh], &[]);
        assert!(glb.len() >= 12);

        let (json_bytes, bin_bytes) = build_glb_parts(
            &[MeshData {
                positions: vec![0.0f32, 0.0, 0.0, 1.0, 1.0, 1.0],
                indices: Vec::new(),
                normals: None,
                colors: Some(vec![255u8, 0, 0, 255, 0, 255, 0, 255]),
                material_index: None,
                mode: 0,
            }],
            &[],
        );
        let json = String::from_utf8_lossy(&json_bytes);
        assert!(
            json.contains("COLOR_0"),
            "JSON should contain COLOR_0 attribute"
        );
        // BIN should have 8 color bytes
        assert!(bin_bytes.len() >= 8, "BIN should contain color data");
    }

    // -------------------------------------------------------------------------
    // Terrain one-shot test
    // -------------------------------------------------------------------------

    #[test]
    fn test_terrain_to_mesh() {
        let heights = vec![
            0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0, // 3x2 grid
        ];
        let bounds = vec![0.0f64, 0.0, 1.0, 1.0]; // west, south, east, north

        let (positions, indices, normals) = terrain_to_mesh(&heights, 3, 2, &bounds);

        // 3x2 = 6 vertices, each with 3 components
        assert_eq!(positions.len(), 18, "Should have 6 vertices × 3 components");
        // (3-1) × (2-1) × 2 triangles × 3 indices = 12 indices
        assert_eq!(indices.len(), 12, "Should have 4 cells × 2 triangles × 3");
        assert_eq!(normals.len(), 18, "Should have 6 normals × 3 components");

        // All normals should point roughly upward (Y > 0)
        for i in 0..6 {
            assert!(
                normals[i * 3 + 1] > 0.0,
                "Normal {} Y component should be positive, got {}",
                i,
                normals[i * 3 + 1]
            );
        }
    }

    #[test]
    fn test_terrain_to_glb_output() {
        let heights = vec![
            10.0f32, 20.0, 10.0, 20.0, 10.0, 20.0, 10.0, 20.0, // 4x2
        ];
        // Actually use terrain_to_mesh + build_glb
        let bounds = vec![116.0f64, 39.0, 117.0, 40.0];
        let (positions, indices, normals) = terrain_to_mesh(&heights, 4, 2, &bounds);
        let mesh = MeshData {
            positions,
            indices,
            normals: Some(normals),
            colors: None,
            material_index: None,
            mode: 4,
        };

        let glb = build_glb(&[mesh], &[]);
        assert!(glb.len() >= 12);
        assert_eq!(&glb[0..4], b"glTF");

        let (json_bytes, _) = build_glb_parts(
            &[MeshData {
                positions: vec![0.0; 24],
                indices: vec![0; 12],
                normals: Some(vec![0.0; 24]),
                colors: None,
                material_index: None,
                mode: 4,
            }],
            &[],
        );
        let json = String::from_utf8_lossy(&json_bytes);
        assert!(json.contains("\"mode\":4"), "Should be TRIANGLES mode (4)");
        assert!(json.contains("NORMAL"), "Should include NORMAL attribute");
    }

    // -------------------------------------------------------------------------
    // Mesh one-shot test
    // -------------------------------------------------------------------------

    #[test]
    fn test_mesh_to_glb() {
        let mesh = MeshData {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
            indices: vec![0, 1, 2],
            normals: Some(vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0]),
            colors: None,
            material_index: None,
            mode: 4,
        };

        let glb = build_glb(&[mesh], &[]);
        assert!(glb.len() >= 12);
        assert_eq!(&glb[0..4], b"glTF");

        let (json_bytes, _) = build_glb_parts(
            &[MeshData {
                positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
                indices: vec![0, 1, 2],
                normals: Some(vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0]),
                colors: None,
                material_index: None,
                mode: 4,
            }],
            &[],
        );
        let json = String::from_utf8_lossy(&json_bytes);
        assert!(json.contains("NORMAL"));
        assert!(json.contains("POSITION"));
    }

    // -------------------------------------------------------------------------
    // Helper for assembling GLB from parts
    // -------------------------------------------------------------------------

    fn assemble_glb(json_bytes: &[u8], bin_bytes: &[u8]) -> Vec<u8> {
        let total_length = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();
        let mut glb = Vec::with_capacity(total_length);
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total_length as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(json_bytes);
        glb.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"BIN\0");
        glb.extend_from_slice(bin_bytes);
        glb
    }

    // -------------------------------------------------------------------------
    // Color attribute tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_builder_mesh_with_colors_and_normals() {
        let mesh = MeshData {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            indices: vec![0, 1],
            normals: Some(vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0]),
            colors: Some(vec![255, 0, 0, 255, 0, 255, 0, 255]),
            material_index: None,
            mode: 4,
        };

        let (json_bytes, bin_bytes) = build_glb_parts(&[mesh], &[]);
        let json = String::from_utf8_lossy(&json_bytes);
        assert!(json.contains("POSITION"));
        assert!(json.contains("NORMAL"));
        assert!(json.contains("COLOR_0"));
        // BIN should contain positions(8) + normals(8) + colors(8) + indices(8) = 32
        assert!(
            bin_bytes.len() >= 32,
            "BIN should contain all attribute data"
        );
    }

    // -------------------------------------------------------------------------
    // GLB alignment tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_glb_4_byte_alignment() {
        // Use an odd-sized mesh to verify padding
        let mesh = MeshData {
            positions: vec![0.0, 1.0, 2.0], // 12 bytes
            indices: vec![0],               // 4 bytes
            normals: None,
            colors: None,
            material_index: None,
            mode: 0,
        };

        let (json_bytes, bin_bytes) = build_glb_parts(&[mesh], &[]);
        assert!(
            json_bytes.len() % 4 == 0,
            "JSON should be 4-byte aligned, len={}",
            json_bytes.len()
        );
        assert!(
            bin_bytes.len() % 4 == 0,
            "BIN should be 4-byte aligned, len={}",
            bin_bytes.len()
        );
    }

    // -------------------------------------------------------------------------
    // Multiple meshes
    // -------------------------------------------------------------------------

    #[test]
    fn test_multiple_meshes() {
        let meshes = vec![
            MeshData {
                positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
                indices: vec![0, 1, 2],
                normals: None,
                colors: None,
                material_index: None,
                mode: 4,
            },
            MeshData {
                positions: vec![10.0, 10.0, 10.0, 11.0, 10.0, 10.0],
                indices: Vec::new(),
                normals: None,
                colors: None,
                material_index: None,
                mode: 0,
            },
        ];

        let (json_bytes, _) = build_glb_parts(&meshes, &[]);
        let json = String::from_utf8_lossy(&json_bytes);
        // Should have 2 meshes and 2 nodes
        assert!(json.contains("\"meshes\""), "Should have meshes");
        assert!(json.contains("\"nodes\""), "Should have nodes");
    }
}
