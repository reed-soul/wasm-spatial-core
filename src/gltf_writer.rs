//! glTF / GLB Writer
//!
//! Build glTF 2.0 scenes programmatically in WASM and export as
//! binary GLB or standalone JSON + BIN. Geometry is consumed as
//! flat typed arrays directly from JS with zero-copy transfer.

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
    mode: Option<u32>, // 4 = TRIANGLES
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
struct GltfMaterial {
    #[serde(rename = "pbrMetallicRoughness")]
    pbr_metallic_roughness: GltfPbr,
    #[serde(rename = "doubleSided")]
    double_sided: Option<bool>,
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

struct MeshData {
    positions: Vec<f32>,
    indices: Vec<u32>,
    normals: Option<Vec<f32>>,
    material_index: Option<usize>,
}

/// glTF 2.0 builder — collect meshes and materials, then export as GLB or JSON.
#[wasm_bindgen]
pub struct GltfBuilder {
    meshes: Vec<MeshData>,
    materials: Vec<GltfMaterial>,
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
            material_index: if material_index >= 0 {
                Some(material_index as usize)
            } else {
                None
            },
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
        });
        idx
    }

    /// Export as binary GLB (`Uint8Array`).
    #[wasm_bindgen(js_name = "toGlb")]
    pub fn to_glb(&self) -> js_sys::Uint8Array {
        let (json_bytes, bin_bytes) = self.build_glb_data();
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

        let arr = js_sys::Uint8Array::new_with_length(glb.len() as u32);
        arr.copy_from(&glb);
        arr
    }

    /// Export as glTF JSON string (no binary — positions/indices as base64).
    #[wasm_bindgen(js_name = "toGltfJson")]
    pub fn to_gltf_json(&self) -> String {
        let (json_bytes, _bin_bytes) = self.build_glb_data();
        String::from_utf8_lossy(&json_bytes).to_string()
    }
}

impl GltfBuilder {
    fn build_glb_data(&self) -> (Vec<u8>, Vec<u8>) {
        let mut accessors = Vec::new();
        let mut buffer_views = Vec::new();
        let mut gltf_meshes = Vec::new();
        let mut gltf_nodes = Vec::new();
        let mut bin_data: Vec<u8> = Vec::new();
        let mut bin_offset: u32 = 0;

        for (mesh_idx, mesh) in self.meshes.iter().enumerate() {
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

            bin_data.extend_from_slice(bytemuck::cast_slice(&mesh.positions));
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

                let norm_acc_idx = accessors.len() as u32;
                accessors.push(GltfAccessor {
                    buffer_view: norm_bv_idx,
                    byte_offset: 0,
                    component_type: 5126,
                    count: vertex_count,
                    accessor_type: "VEC3".to_string(),
                    min: None,
                    max: None,
                });

                bin_data.extend_from_slice(bytemuck::cast_slice(normals));
                bin_offset += norm_bytes;

                // Will be added to attributes below
                let _ = norm_acc_idx;
            }

            // ── Index accessor + buffer view ──────────────────────
            let idx_bytes = mesh.indices.len() as u32 * 4;
            let idx_bv_idx = buffer_views.len() as u32;
            buffer_views.push(GltfBufferView {
                buffer: 0,
                byte_offset: bin_offset,
                byte_length: idx_bytes,
                target: Some(34963), // ELEMENT_ARRAY_BUFFER
            });

            let _idx_acc_idx = accessors.len() as u32;
            accessors.push(GltfAccessor {
                buffer_view: idx_bv_idx,
                byte_offset: 0,
                component_type: 5125, // UNSIGNED_INT
                count: mesh.indices.len() as u32,
                accessor_type: "SCALAR".to_string(),
                min: None,
                max: None,
            });

            bin_data.extend_from_slice(bytemuck::cast_slice(&mesh.indices));
            bin_offset += idx_bytes;

            // ── Build primitive attributes map ─────────────────────
            let mut attrs = serde_json::Map::new();
            attrs.insert("POSITION".to_string(), serde_json::json!(pos_acc_idx));

            // If normals exist, compute accessor index and add attribute
            if mesh.normals.is_some() {
                // Normal accessor was pushed after position accessor
                // pos_acc_idx + 1 is the normal accessor index
                attrs.insert("NORMAL".to_string(), serde_json::json!(pos_acc_idx + 1));
            }

            primitives.push(GltfPrimitive {
                attributes: attrs,
                mode: Some(4), // TRIANGLES
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
            materials: self.materials.clone(),
        };

        let json_str = serde_json::to_string(&root).unwrap_or_default();
        let mut json_bytes = json_str.into_bytes();

        // Pad JSON to 4-byte alignment with spaces (0x20)
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(0x20);
        }

        (json_bytes, bin_data)
    }
}

// Minimal bytemuck-compatible cast
mod bytemuck {
    pub fn cast_slice<T: Copy>(slice: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice))
        }
    }
}

// ===========================================================================
// Tests (core logic only — no WASM runtime dependency)
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
            material_index: if material_index >= 0 {
                Some(material_index as usize)
            } else {
                None
            },
        });
    }

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

        let (json_bytes, bin_bytes) = builder.build_glb_data();
        let total_length = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();

        // Build GLB manually
        let mut glb: Vec<u8> = Vec::with_capacity(total_length);
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total_length as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(&json_bytes);
        glb.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"BIN\0");
        glb.extend_from_slice(&bin_bytes);

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

        let (json_bytes, bin_bytes) = builder.build_glb_data();
        let total_length = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();

        let mut glb: Vec<u8> = Vec::with_capacity(total_length);
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total_length as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(&json_bytes);
        glb.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"BIN\0");
        glb.extend_from_slice(&bin_bytes);

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

        let (json_bytes, _) = builder.build_glb_data();
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

        let (json_bytes, bin_bytes) = builder.build_glb_data();
        let total_length = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();
        let mut glb: Vec<u8> = Vec::with_capacity(total_length);
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total_length as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(&json_bytes);
        glb.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"BIN\0");
        glb.extend_from_slice(&bin_bytes);

        assert!(!glb.is_empty());
        assert_eq!(&glb[0..4], b"glTF");

        let json = String::from_utf8_lossy(&json_bytes);
        assert!(
            json.contains("\"materials\""),
            "JSON should contain materials"
        );
    }
}
