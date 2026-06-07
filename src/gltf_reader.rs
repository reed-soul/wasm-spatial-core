//! glTF / GLB reader — parse binary GLB into Spatial IR [`MeshChunk`].
//!
//! Mirrors the subset produced by [`crate::gltf_writer`]: TRIANGLES / POINTS
//! primitives with POSITION, NORMAL, and UNSIGNED_INT indices.

use serde::Deserialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::spatial_ir::{Aabb, ChunkMeta, MeshChunk, WasmMeshChunk};
use crate::validate_input_size;

const GLB_MAGIC: &[u8; 4] = b"glTF";
const CHUNK_JSON: &[u8; 4] = b"JSON";
const CHUNK_BIN: &[u8; 4] = b"BIN\0";

const COMPONENT_FLOAT: u32 = 5126;
const COMPONENT_UNSIGNED_INT: u32 = 5125;
const COMPONENT_UNSIGNED_SHORT: u32 = 5123;

// ===========================================================================
// glTF JSON model (deserialize subset)
// ===========================================================================

#[derive(Debug, Deserialize)]
struct GltfRoot {
    #[serde(default)]
    meshes: Vec<GltfMesh>,
    #[serde(default)]
    accessors: Vec<GltfAccessor>,
    #[serde(rename = "bufferViews", default)]
    buffer_views: Vec<GltfBufferView>,
    #[serde(default)]
    extensions_used: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GltfMesh {
    primitives: Vec<GltfPrimitive>,
}

#[derive(Debug, Deserialize)]
struct GltfPrimitive {
    attributes: HashMap<String, u32>,
    indices: Option<u32>,
    mode: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GltfAccessor {
    #[serde(rename = "bufferView")]
    buffer_view: Option<u32>,
    #[serde(rename = "byteOffset", default)]
    byte_offset: u32,
    #[serde(rename = "componentType")]
    component_type: u32,
    count: u32,
    #[serde(rename = "type")]
    accessor_type: String,
}

#[derive(Debug, Deserialize)]
struct GltfBufferView {
    #[serde(rename = "byteOffset", default)]
    byte_offset: u32,
    #[serde(rename = "byteLength")]
    byte_length: u32,
}

struct GlbChunks<'a> {
    json: &'a [u8],
    bin: &'a [u8],
}

// ===========================================================================
// Core parser
// ===========================================================================

/// Parse a GLB file into a merged [`MeshChunk`].
///
/// Multiple mesh primitives are concatenated with remapped indices.
pub fn parse_glb_core(bytes: &[u8]) -> Result<MeshChunk, SpatialErrorDetail> {
    let chunks = split_glb(bytes)?;
    let root: GltfRoot = serde_json::from_slice(chunks.json)
        .map_err(|e| SpatialError::ParseError.with_detail(format!("invalid glTF JSON: {e}")))?;

    if !root.extensions_used.is_empty() {
        return Err(SpatialError::InvalidInput.with_detail(format!(
            "unsupported glTF extensions: {}",
            root.extensions_used.join(", ")
        )));
    }

    if root.meshes.is_empty() {
        return Err(SpatialError::InvalidInput.with_detail("GLB contains no meshes"));
    }

    let mut merged_positions = Vec::new();
    let mut merged_indices = Vec::new();
    let mut merged_normals: Option<Vec<f32>> = None;
    let mut mode = MeshChunk::MODE_TRIANGLES;

    for mesh in &root.meshes {
        for primitive in &mesh.primitives {
            let prim_mode = primitive.mode.unwrap_or(MeshChunk::MODE_TRIANGLES);
            if merged_positions.is_empty() {
                mode = prim_mode;
            } else if mode != prim_mode {
                return Err(SpatialError::InvalidInput
                    .with_detail("mixed primitive modes in one GLB are not supported"));
            }

            let vertex_offset = (merged_positions.len() / 3) as u32;
            let positions = read_vec3_f32(
                &root,
                chunks.bin,
                primitive.attributes.get("POSITION").copied(),
            )?;
            let normals = match primitive.attributes.get("NORMAL") {
                Some(idx) => Some(read_vec3_f32(&root, chunks.bin, Some(*idx))?),
                None => None,
            };

            if let Some(ref n) = normals {
                if n.len() != positions.len() {
                    return Err(SpatialError::ParseError
                        .with_detail("NORMAL count does not match POSITION count"));
                }
            }

            merged_positions.extend_from_slice(&positions);

            match &mut merged_normals {
                None if normals.is_some() => merged_normals = normals,
                Some(acc) => {
                    if let Some(n) = normals {
                        acc.extend_from_slice(&n);
                    } else {
                        acc.extend(std::iter::repeat_n(0.0, positions.len()));
                    }
                }
                None => {}
            }

            if let Some(indices_accessor) = primitive.indices {
                let indices = read_indices(&root, chunks.bin, indices_accessor)?;
                merged_indices.extend(indices.into_iter().map(|i| i + vertex_offset));
            } else if prim_mode == MeshChunk::MODE_TRIANGLES {
                let vertex_count = positions.len() / 3;
                merged_indices.extend((0..vertex_count as u32).map(|i| i + vertex_offset));
            }
        }
    }

    if merged_positions.is_empty() {
        return Err(SpatialError::InvalidInput.with_detail("GLB mesh has no vertices"));
    }

    if mode == MeshChunk::MODE_TRIANGLES && merged_indices.is_empty() {
        return Err(SpatialError::ParseError.with_detail("triangle mesh has no indices"));
    }

    let aabb = Aabb::from_positions(&merged_positions);
    let byte_budget = merged_positions.len() * 4
        + merged_indices.len() * 4
        + merged_normals.as_ref().map_or(0, |n| n.len() * 4);

    let mut metadata = ChunkMeta::new("glb").with_aabb(aabb);
    metadata.byte_budget = Some(byte_budget);

    Ok(MeshChunk {
        metadata,
        positions: merged_positions,
        indices: merged_indices,
        normals: merged_normals,
        texcoords: None,
        mode,
    })
}

fn split_glb(bytes: &[u8]) -> Result<GlbChunks<'_>, SpatialErrorDetail> {
    if bytes.len() < 12 {
        return Err(SpatialError::InvalidInput.with_detail("GLB too small"));
    }
    if &bytes[0..4] != GLB_MAGIC {
        return Err(SpatialError::InvalidInput.with_detail("invalid GLB magic"));
    }

    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != 2 {
        return Err(
            SpatialError::InvalidInput.with_detail(format!("unsupported GLB version: {version}"))
        );
    }

    let total_length = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    if total_length != bytes.len() {
        return Err(SpatialError::InvalidInput.with_detail(format!(
            "GLB length mismatch: header={total_length}, actual={}",
            bytes.len()
        )));
    }

    let mut offset = 12;
    let mut json: Option<&[u8]> = None;
    let mut bin: &[u8] = &[];

    while offset + 8 <= bytes.len() {
        let chunk_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        let chunk_type = &bytes[offset + 4..offset + 8];
        offset += 8;

        if offset + chunk_len > bytes.len() {
            return Err(SpatialError::ParseError.with_detail("truncated GLB chunk"));
        }

        let chunk_data = &bytes[offset..offset + chunk_len];
        offset += chunk_len;

        if chunk_type == CHUNK_JSON {
            json = Some(chunk_data);
        } else if chunk_type == CHUNK_BIN {
            bin = chunk_data;
        }
    }

    let json =
        json.ok_or_else(|| SpatialError::ParseError.with_detail("GLB missing JSON chunk"))?;
    Ok(GlbChunks { json, bin })
}

fn accessor_bytes<'a>(
    root: &GltfRoot,
    bin: &'a [u8],
    accessor_index: u32,
) -> Result<&'a [u8], SpatialErrorDetail> {
    let accessor = root
        .accessors
        .get(accessor_index as usize)
        .ok_or_else(|| SpatialError::ParseError.with_detail("accessor index out of range"))?;

    let buffer_view_index = accessor.buffer_view.ok_or_else(|| {
        SpatialError::ParseError.with_detail("accessor has no bufferView (sparse not supported)")
    })?;

    let buffer_view = root
        .buffer_views
        .get(buffer_view_index as usize)
        .ok_or_else(|| SpatialError::ParseError.with_detail("bufferView index out of range"))?;

    let start = buffer_view.byte_offset as usize + accessor.byte_offset as usize;
    let end = start + buffer_view.byte_length as usize;
    if end > bin.len() {
        return Err(SpatialError::ParseError.with_detail("bufferView extends past BIN chunk"));
    }

    Ok(&bin[start..end])
}

fn read_vec3_f32(
    root: &GltfRoot,
    bin: &[u8],
    accessor_index: Option<u32>,
) -> Result<Vec<f32>, SpatialErrorDetail> {
    let accessor_index =
        accessor_index.ok_or_else(|| SpatialError::ParseError.with_detail("missing accessor"))?;
    let accessor = root
        .accessors
        .get(accessor_index as usize)
        .ok_or_else(|| SpatialError::ParseError.with_detail("accessor index out of range"))?;

    if accessor.accessor_type != "VEC3" {
        return Err(SpatialError::ParseError.with_detail(format!(
            "expected VEC3 accessor, got {}",
            accessor.accessor_type
        )));
    }
    if accessor.component_type != COMPONENT_FLOAT {
        return Err(SpatialError::ParseError.with_detail("POSITION/NORMAL must be FLOAT (5126)"));
    }

    let raw = accessor_bytes(root, bin, accessor_index)?;
    let expected = accessor.count as usize * 12;
    if raw.len() < expected {
        return Err(SpatialError::ParseError.with_detail("accessor data too short"));
    }

    let mut out = Vec::with_capacity(accessor.count as usize * 3);
    for i in 0..accessor.count as usize {
        let base = i * 12;
        out.push(f32::from_le_bytes([
            raw[base],
            raw[base + 1],
            raw[base + 2],
            raw[base + 3],
        ]));
        out.push(f32::from_le_bytes([
            raw[base + 4],
            raw[base + 5],
            raw[base + 6],
            raw[base + 7],
        ]));
        out.push(f32::from_le_bytes([
            raw[base + 8],
            raw[base + 9],
            raw[base + 10],
            raw[base + 11],
        ]));
    }
    Ok(out)
}

fn read_indices(
    root: &GltfRoot,
    bin: &[u8],
    accessor_index: u32,
) -> Result<Vec<u32>, SpatialErrorDetail> {
    let accessor = root
        .accessors
        .get(accessor_index as usize)
        .ok_or_else(|| SpatialError::ParseError.with_detail("indices accessor out of range"))?;

    if accessor.accessor_type != "SCALAR" {
        return Err(SpatialError::ParseError.with_detail("indices accessor must be SCALAR"));
    }

    let raw = accessor_bytes(root, bin, accessor_index)?;
    let count = accessor.count as usize;

    match accessor.component_type {
        COMPONENT_UNSIGNED_INT => {
            if raw.len() < count * 4 {
                return Err(SpatialError::ParseError.with_detail("index data too short"));
            }
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                let base = i * 4;
                out.push(u32::from_le_bytes([
                    raw[base],
                    raw[base + 1],
                    raw[base + 2],
                    raw[base + 3],
                ]));
            }
            Ok(out)
        }
        COMPONENT_UNSIGNED_SHORT => {
            if raw.len() < count * 2 {
                return Err(SpatialError::ParseError.with_detail("index data too short"));
            }
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                let base = i * 2;
                out.push(u16::from_le_bytes([raw[base], raw[base + 1]]) as u32);
            }
            Ok(out)
        }
        other => Err(SpatialError::ParseError
            .with_detail(format!("unsupported index component type: {other}"))),
    }
}

// ===========================================================================
// WASM API
// ===========================================================================

/// Parse a GLB file into a [`WasmMeshChunk`].
#[wasm_bindgen(js_name = "parseGlb")]
pub fn parse_glb(bytes: &[u8]) -> Result<WasmMeshChunk, JsValue> {
    validate_input_size(bytes.len(), "GLB")?;
    parse_glb_core(bytes)
        .map(WasmMeshChunk::from_chunk)
        .map_err(Into::into)
}

/// Whether mesh ingest (Spatial IR + GLB read) is available.
#[wasm_bindgen(js_name = "supportsMeshIngest")]
pub fn supports_mesh_ingest() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gltf_writer::{build_glb, MeshData};

    fn triangle_mesh() -> MeshData {
        MeshData {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
            indices: vec![0, 1, 2],
            normals: Some(vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0]),
            colors: None,
            material_index: None,
            mode: 4,
        }
    }

    #[test]
    fn test_roundtrip_mesh_to_glb_parse_glb() {
        let glb = build_glb(&[triangle_mesh()], &[]);
        let chunk = parse_glb_core(&glb).unwrap();
        assert_eq!(chunk.vertex_count(), 3);
        assert_eq!(chunk.indices.len(), 3);
        assert!(chunk.normals.is_some());
        assert_eq!(chunk.metadata.source_format, Some("glb".to_string()));
    }

    #[test]
    fn test_roundtrip_preserves_positions() {
        let source = triangle_mesh();
        let glb = build_glb(&[source], &[]);
        let chunk = parse_glb_core(&glb).unwrap();
        assert_eq!(
            chunk.positions,
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0]
        );
        assert_eq!(chunk.indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_invalid_magic() {
        let err = parse_glb_core(b"not-a-glb").unwrap_err();
        assert_eq!(err.code(), SpatialError::InvalidInput.code());
    }

    #[test]
    fn test_point_cloud_mode_no_indices() {
        let mesh = MeshData {
            positions: vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
            indices: Vec::new(),
            normals: None,
            colors: None,
            material_index: None,
            mode: 0,
        };
        let glb = build_glb(&[mesh], &[]);
        let chunk = parse_glb_core(&glb).unwrap();
        assert_eq!(chunk.vertex_count(), 2);
        assert!(chunk.indices.is_empty());
        assert_eq!(chunk.mode, 0);
    }

    #[test]
    fn test_select_after_parse() {
        let glb = build_glb(&[triangle_mesh()], &[]);
        let chunk = parse_glb_core(&glb).unwrap();
        let region = Aabb {
            min: [-0.1, -0.1, -0.1],
            max: [0.6, 1.1, 0.1],
        };
        let selected = chunk.select_by_aabb(&region).unwrap();
        assert_eq!(selected.vertex_count(), 3);
    }
}
