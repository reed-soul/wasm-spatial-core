//! COPC octree hierarchy parsing and spatial chunk selection (laz-support).
//!
//! Implements the hierarchy page walk described in the
//! [COPC 1.0 specification](https://copc.io/).

use std::collections::HashSet;
use std::collections::VecDeque;

/// Parsed COPC `info` VLR payload (160 bytes).
#[derive(Debug, Clone, Copy)]
pub struct CopcInfoVlrData {
    pub center_x: f64,
    pub center_y: f64,
    pub center_z: f64,
    pub halfsize: f64,
    #[allow(dead_code)]
    pub spacing: f64,
    pub root_hier_offset: u64,
    pub root_hier_size: u64,
}

/// Axis-aligned bounding box (min/max per axis).
#[derive(Debug, Clone, Copy)]
pub struct Bbox3d {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

/// One 32-byte hierarchy `Entry` (data chunk or child page pointer).
#[derive(Debug, Clone, Copy)]
pub struct CopcHierarchyEntry {
    pub level: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub offset: u64,
    pub byte_size: i32,
    pub point_count: i32,
}

/// A COPC data chunk selected for decompression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopcDataChunk {
    pub offset: u64,
    pub byte_size: u64,
    pub point_count: u32,
}

/// Parse the 160-byte COPC `info` VLR payload.
pub fn parse_copc_info_vlr(data: &[u8]) -> Result<CopcInfoVlrData, String> {
    if data.len() < 160 {
        return Err(format!(
            "COPC info VLR too short: {} bytes (expected 160)",
            data.len()
        ));
    }
    Ok(CopcInfoVlrData {
        center_x: f64::from_le_bytes(data[0..8].try_into().unwrap()),
        center_y: f64::from_le_bytes(data[8..16].try_into().unwrap()),
        center_z: f64::from_le_bytes(data[16..24].try_into().unwrap()),
        halfsize: f64::from_le_bytes(data[24..32].try_into().unwrap()),
        spacing: f64::from_le_bytes(data[32..40].try_into().unwrap()),
        root_hier_offset: u64::from_le_bytes(data[40..48].try_into().unwrap()),
        root_hier_size: u64::from_le_bytes(data[48..56].try_into().unwrap()),
    })
}

/// Parse one hierarchy page into entries.
pub fn parse_hierarchy_page(
    bytes: &[u8],
    offset: u64,
    size: u64,
) -> Result<Vec<CopcHierarchyEntry>, String> {
    // Use checked conversions: `as usize` would silently truncate on 32-bit
    // targets for offsets/sizes > usize::MAX (4 GiB), producing wrong slices.
    let start = usize::try_from(offset)
        .map_err(|_| format!("COPC hierarchy page offset {} exceeds usize", offset))?;
    let sz = usize::try_from(size)
        .map_err(|_| format!("COPC hierarchy page size {} exceeds usize", size))?;
    let end = start
        .checked_add(sz)
        .ok_or_else(|| "COPC hierarchy page size overflow".to_string())?;
    if end > bytes.len() {
        return Err(format!(
            "COPC hierarchy page extends beyond file (offset={}, size={}, file_len={})",
            offset,
            size,
            bytes.len()
        ));
    }
    if !size.is_multiple_of(32) {
        return Err(format!(
            "COPC hierarchy page size {} is not a multiple of 32",
            size
        ));
    }
    let page = &bytes[start..end];
    let mut entries = Vec::with_capacity(page.len() / 32);
    for chunk in page.chunks_exact(32) {
        entries.push(CopcHierarchyEntry {
            level: i32::from_le_bytes(chunk[0..4].try_into().unwrap()),
            x: i32::from_le_bytes(chunk[4..8].try_into().unwrap()),
            y: i32::from_le_bytes(chunk[8..12].try_into().unwrap()),
            z: i32::from_le_bytes(chunk[12..16].try_into().unwrap()),
            offset: u64::from_le_bytes(chunk[16..24].try_into().unwrap()),
            byte_size: i32::from_le_bytes(chunk[24..28].try_into().unwrap()),
            point_count: i32::from_le_bytes(chunk[28..32].try_into().unwrap()),
        });
    }
    Ok(entries)
}

/// Voxel axis-aligned bounds for a hierarchy key.
pub fn voxel_bounds(info: &CopcInfoVlrData, level: i32, x: i32, y: i32, z: i32) -> Option<Bbox3d> {
    if level < 0 {
        return None;
    }
    let level = level as u32;
    let denom = 1u64.checked_shl(level)?;
    let node_size = (info.halfsize * 2.0) / denom as f64;
    let min_x = info.center_x - info.halfsize + x as f64 * node_size;
    let min_y = info.center_y - info.halfsize + y as f64 * node_size;
    let min_z = info.center_z - info.halfsize + z as f64 * node_size;
    Some(Bbox3d {
        min_x,
        min_y,
        min_z,
        max_x: min_x + node_size,
        max_y: min_y + node_size,
        max_z: min_z + node_size,
    })
}

fn bbox_intersects(a: Bbox3d, b: Bbox3d) -> bool {
    a.min_x <= b.max_x
        && a.max_x >= b.min_x
        && a.min_y <= b.max_y
        && a.max_y >= b.min_y
        && a.min_z <= b.max_z
        && a.max_z >= b.min_z
}

/// Collect data chunks whose voxels intersect the query bounding box.
pub fn query_copc_chunks_for_bbox(
    bytes: &[u8],
    info: &CopcInfoVlrData,
    query: Bbox3d,
) -> Result<Vec<CopcDataChunk>, String> {
    if info.root_hier_size == 0 || info.root_hier_offset == 0 {
        return Ok(Vec::new());
    }

    let mut chunks = Vec::new();
    let mut queue = VecDeque::new();
    // Track visited page offsets to prevent unbounded re-enqueue / infinite
    // loops when a malformed or adversarial COPC file has pages that reference
    // already-visited offsets (or self-references) — a denial-of-service hazard.
    let mut visited: HashSet<u64> = HashSet::new();
    queue.push_back((info.root_hier_offset, info.root_hier_size));
    visited.insert(info.root_hier_offset);

    while let Some((page_offset, page_size)) = queue.pop_front() {
        let entries = parse_hierarchy_page(bytes, page_offset, page_size)?;
        for entry in entries {
            if entry.point_count > 0 {
                if let Some(voxel) = voxel_bounds(info, entry.level, entry.x, entry.y, entry.z) {
                    if bbox_intersects(voxel, query) {
                        // Reject invalid non-positive byte_size for chunks that
                        // claim to contain points; `.max(0)` would silently
                        // produce a zero-length slice and break decompression.
                        if entry.byte_size <= 0 {
                            return Err(format!(
                                "COPC chunk at level {} ({},{},{}) has point_count {} but invalid byte_size {}",
                                entry.level, entry.x, entry.y, entry.z,
                                entry.point_count, entry.byte_size
                            ));
                        }
                        chunks.push(CopcDataChunk {
                            offset: entry.offset,
                            byte_size: entry.byte_size as u64,
                            point_count: entry.point_count as u32,
                        });
                    }
                }
            } else if entry.point_count == -1 && entry.byte_size > 0 {
                // Child hierarchy page pointer. Skip if already visited.
                if visited.insert(entry.offset) {
                    queue.push_back((entry.offset, entry.byte_size as u64));
                }
            }
        }
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_copc_info_vlr() {
        let mut data = vec![0u8; 160];
        data[0..8].copy_from_slice(&100.0_f64.to_le_bytes());
        data[8..16].copy_from_slice(&200.0_f64.to_le_bytes());
        data[16..24].copy_from_slice(&50.0_f64.to_le_bytes());
        data[24..32].copy_from_slice(&500.0_f64.to_le_bytes());
        data[40..48].copy_from_slice(&1000u64.to_le_bytes());
        data[48..56].copy_from_slice(&64u64.to_le_bytes());
        let info = parse_copc_info_vlr(&data).unwrap();
        assert_eq!(info.center_x, 100.0);
        assert_eq!(info.root_hier_offset, 1000);
        assert_eq!(info.root_hier_size, 64);
    }

    #[test]
    fn test_parse_hierarchy_page_single_chunk() {
        let mut page = vec![0u8; 32];
        page[16..24].copy_from_slice(&5000u64.to_le_bytes());
        page[24..28].copy_from_slice(&128i32.to_le_bytes());
        page[28..32].copy_from_slice(&10i32.to_le_bytes());
        let mut file = vec![0u8; 100];
        file.extend_from_slice(&page);
        let entries = parse_hierarchy_page(&file, 100, 32).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].point_count, 10);
        assert_eq!(entries[0].offset, 5000);
    }

    #[test]
    fn test_voxel_bounds_root() {
        let info = CopcInfoVlrData {
            center_x: 0.0,
            center_y: 0.0,
            center_z: 0.0,
            halfsize: 100.0,
            spacing: 1.0,
            root_hier_offset: 0,
            root_hier_size: 0,
        };
        let bbox = voxel_bounds(&info, 0, 0, 0, 0).unwrap();
        assert_eq!(bbox.min_x, -100.0);
        assert_eq!(bbox.max_x, 100.0);
        assert_eq!(bbox.min_z, -100.0);
        assert_eq!(bbox.max_z, 100.0);
        assert!((bbox.min_y - -100.0).abs() < 1e-9);
        assert!((bbox.max_y - 100.0).abs() < 1e-9);
    }
}
