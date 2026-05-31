//! GeoTIFF Terrain Decoding & Quantized-Mesh Encoding
//!
//! Hand-written TIFF/GeoTIFF parser — no external TIFF library. Follows the
//! same philosophy as `point_cloud.rs` (hand-written LAS parser): the format
//! is simple enough that we parse it directly, keeping the WASM binary small
//! and avoiding native-only dependencies.
//!
//! Supports:
//! - Uncompressed and DEFLATE (ZLib) compressed TIFF data
//! - Float32 elevation grids (SingleBand, SampleFormat=IEEEFP)
//! - Strip-organized and Tile-organized layouts
//! - GeoKey metadata (CRS, ModelType, etc.)
//!
//! Outputs:
//! - `GeotiffInfo` — parsed metadata + elevation access
//! - `QuantizedMeshResult` — Cesium quantized-mesh terrain tiles
//! - `TerrainTilesetResult` — tileset.json + quantized-mesh tile pyramid

use flate2::read::ZlibDecoder;
use std::io::Read;
use wasm_bindgen::prelude::*;

// ===========================================================================
// TIFF Constants
// ===========================================================================

/// TIFF tag IDs
#[allow(dead_code)]
mod tag {
    pub const IMAGE_WIDTH: u16 = 256;
    pub const IMAGE_LENGTH: u16 = 257;
    pub const BITS_PER_SAMPLE: u16 = 258;
    pub const COMPRESSION: u16 = 259;
    pub const STRIP_OFFSETS: u16 = 273;
    pub const ROWS_PER_STRIP: u16 = 278;
    pub const STRIP_BYTE_COUNTS: u16 = 279;
    pub const TILE_WIDTH: u16 = 322;
    pub const TILE_LENGTH: u16 = 323;
    pub const TILE_OFFSETS: u16 = 324;
    pub const TILE_BYTE_COUNTS: u16 = 325;
    pub const SAMPLE_FORMAT: u16 = 339;
    // GeoTIFF tags
    pub const GEO_KEY_DIRECTORY: u16 = 34735;
    pub const GEO_DOUBLE_PARAMS: u16 = 34736;
    pub const GEO_ASCII_PARAMS: u16 = 34737;
}

/// TIFF field type IDs
#[allow(dead_code)]
mod field_type {
    pub const BYTE: u16 = 1;
    pub const ASCII: u16 = 2;
    pub const SHORT: u16 = 3;
    pub const LONG: u16 = 4;
    pub const RATIONAL: u16 = 5;
    pub const SBYTE: u16 = 6;
    pub const UNDEFINED: u16 = 7;
    pub const SSHORT: u16 = 8;
    pub const SLONG: u16 = 9;
    pub const SRATIONAL: u16 = 10;
    pub const FLOAT: u16 = 11;
    pub const DOUBLE: u16 = 12;
}

/// Compression method codes
#[allow(dead_code)]
mod compression {
    pub const NONE: u16 = 1;
    pub const CCITT_RLE: u16 = 2;
    pub const LZW: u16 = 5;
    pub const DEFLATE: u16 = 8; // Adobe-style DEFLATE
    pub const DEFLATE_GEOTIFF: u16 = 34712; // GeoTIFF-style DEFLATE
}

/// TIFF field type byte sizes
fn field_type_size(type_id: u16) -> usize {
    match type_id {
        field_type::BYTE | field_type::ASCII | field_type::SBYTE | field_type::UNDEFINED => 1,
        field_type::SHORT | field_type::SSHORT => 2,
        field_type::LONG | field_type::SLONG | field_type::FLOAT => 4,
        field_type::RATIONAL | field_type::SRATIONAL | field_type::DOUBLE => 8,
        _ => 0,
    }
}

// ===========================================================================
// GeoKey Constants
// ===========================================================================

#[allow(dead_code)]
mod geokey {
    pub const GT_MODEL_TYPE_GEO_KEY: u16 = 1024;
    pub const GT_RASTER_TYPE_GEO_KEY: u16 = 1025;
    pub const GT_CITATION_GEO_KEY: u16 = 1026;
    pub const GEOGRAPHIC_TYPE_GEO_KEY: u16 = 2048;
    pub const GEOGRAPHIC_CITATION_GEO_KEY: u16 = 2049;
    pub const GEOG_GEODETIC_DATUM_GEO_KEY: u16 = 2050;
    pub const GEOG_PRIME_MERIDIAN_GEO_KEY: u16 = 2051;
    pub const GEOG_LINEAR_UNITS_GEO_KEY: u16 = 2052;
    pub const GEOG_LINEAR_UNIT_SIZE_GEO_KEY: u16 = 2053;
    pub const GEOG_ANGULAR_UNITS_GEO_KEY: u16 = 2054;
    pub const GEOG_ANGULAR_UNIT_SIZE_GEO_KEY: u16 = 2055;
    pub const GEOG_ELLIPSOID_GEO_KEY: u16 = 2056;
    pub const GEOG_SEMI_MAJOR_AXIS_GEO_KEY: u16 = 2057;
    pub const GEOG_SEMI_MINOR_AXIS_GEO_KEY: u16 = 2058;
    pub const GEOG_INV_FLATTENING_GEO_KEY: u16 = 2059;
    pub const GEOG_AZIMUTH_UNITS_GEO_KEY: u16 = 2060;
    pub const GEOG_PRIME_MERIDIAN_LONG_GEO_KEY: u16 = 2061;
    pub const PROJECTED_CST_TYPE_GEO_KEY: u16 = 3072;
    pub const PROJECTED_CITATION_GEO_KEY: u16 = 3073;
    pub const PROJECTION_GEO_KEY: u16 = 3074;
    pub const PROJ_COORD_TRANS_GEO_KEY: u16 = 3075;
    pub const PROJ_LINEAR_UNITS_GEO_KEY: u16 = 3076;
    pub const PROJ_LINEAR_UNIT_SIZE_GEO_KEY: u16 = 3077;
    pub const PROJ_STD_PARALLEL_1_GEO_KEY: u16 = 3078;
    pub const PROJ_STD_PARALLEL_2_GEO_KEY: u16 = 3079;
    pub const PROJ_NAT_ORIGIN_LONG_GEO_KEY: u16 = 3080;
    pub const PROJ_NAT_ORIGIN_LAT_GEO_KEY: u16 = 3081;
    pub const PROJ_FALSE_EASTING_GEO_KEY: u16 = 3082;
    pub const PROJ_FALSE_NORTHING_GEO_KEY: u16 = 3083;
    pub const PROJ_RECTIFIED_GRID_ANGLE_GEO_KEY: u16 = 3084;
    pub const PROJ_SCALE_AT_NAT_ORIGIN_GEO_KEY: u16 = 3085;
    pub const PROJ_HEIGHT_OF_PROJ_ORG_GEO_KEY: u16 = 3086;
    pub const VERTICAL_CST_TYPE_GEO_KEY: u16 = 4096;
}

/// GTModelTypeGeoKey values
#[allow(dead_code)]
mod model_type {
    pub const PROJECTED: u16 = 1;
    pub const GEOGRAPHIC: u16 = 2;
    pub const GEOCENTRIC: u16 = 3;
}

/// Common CRS codes
#[allow(dead_code)]
mod crs_code {
    pub const WGS_84: u16 = 4326;
    pub const WGS_72: u16 = 4322;
    pub const NAD_83: u16 = 4269;
}

// ===========================================================================
// Data Structures
// ===========================================================================

/// Byte order of a TIFF file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ByteOrder {
    LittleEndian,
    BigEndian,
}

/// A single IFD entry (tag).
#[derive(Debug, Clone)]
struct IfdEntry {
    tag_id: u16,
    field_type: u16,
    count: u32,
    // If total value fits in 4 bytes, stored inline; otherwise an offset.
    // We store the actual byte offset into the file for BOTH cases.
    value_bytes_offset: usize, // byte position of the 4-byte value/offset field
    value_u32: u32,            // raw 4-byte value read from the field
}

/// Parsed GeoTIFF metadata.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GeotiffInternal {
    byte_order: ByteOrder,
    image_width: u32,
    image_length: u32,
    bits_per_sample: u16,
    compression: u16,
    sample_format: u16,
    /// Strip offsets (for strip-organized images)
    strip_offsets: Vec<u32>,
    strip_byte_counts: Vec<u64>,
    rows_per_strip: u32,
    /// Tile layout (for tiled images)
    tile_width: u32,
    tile_length: u32,
    tile_offsets: Vec<u64>,
    tile_byte_counts: Vec<u64>,
    /// GeoTIFF keys
    geo_keys: Vec<(u16, u16, u16, StringOrShort)>, // (tag_id, tiff_tag_loc, count, value)
    geo_double_params: Vec<f64>,
    geo_ascii_params: String,
    is_tiled: bool,
}

/// GeoKey value — either a short value or string reference into GeoAsciiParams.
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum StringOrShort {
    Short(u16),
    #[allow(dead_code)]
    AsciiOffset(#[allow(dead_code)] usize, #[allow(dead_code)] usize), // start, length
}

/// Parsed GeoTIFF ready for WASM consumption.
#[wasm_bindgen]
#[derive(Debug)]
pub struct GeotiffInfo {
    inner: GeotiffInternal,
    /// Elevation data (flat f32 array, row-major, width*height)
    elevations: Vec<f32>,
    /// Geo bounds: [min_lng, min_lat, max_lng, max_lat]
    bounds: [f64; 4],
}

#[wasm_bindgen]
impl GeotiffInfo {
    /// Image width in pixels.
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.inner.image_width
    }

    /// Image height in pixels.
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.inner.image_length
    }

    /// Elevation values as Float32Array (row-major, width*height).
    #[wasm_bindgen(getter)]
    pub fn elevation(&self) -> js_sys::Float32Array {
        let arr = js_sys::Float32Array::new_with_length(self.elevations.len() as u32);
        arr.copy_from(&self.elevations);
        arr
    }

    /// Get elevation for a specific strip (swath). Returns Float32Array.
    /// For strip-organized images, swath_index selects a strip.
    /// For the full elevation grid, just use `elevation()`.
    #[wasm_bindgen(js_name = "elevationSwath")]
    pub fn elevation_swath(&self, swath_index: usize) -> js_sys::Float32Array {
        let rows_per_strip = self.inner.rows_per_strip as usize;
        let w = self.inner.image_width as usize;
        let start_row = swath_index * rows_per_strip;
        let end_row = (start_row + rows_per_strip).min(self.inner.image_length as usize);
        let start = start_row * w;
        let end = end_row * w;
        if start >= self.elevations.len() {
            return js_sys::Float32Array::new_with_length(0);
        }
        let slice = &self.elevations[start..end.min(self.elevations.len())];
        let arr = js_sys::Float32Array::new_with_length(slice.len() as u32);
        arr.copy_from(slice);
        arr
    }

    /// Geographic bounds as Float64Array: [min_lng, min_lat, max_lng, max_lat].
    #[wasm_bindgen(getter)]
    pub fn bounds(&self) -> js_sys::Float64Array {
        let arr = js_sys::Float64Array::new_with_length(4);
        arr.copy_from(&self.bounds);
        arr
    }

    /// Resolution in degrees per pixel.
    #[wasm_bindgen(getter)]
    pub fn resolution(&self) -> f64 {
        let lng_range = self.bounds[2] - self.bounds[0];
        lng_range / self.inner.image_width as f64
    }

    /// CRS information as JSON string.
    #[wasm_bindgen(getter)]
    pub fn crs(&self) -> String {
        let mut info = serde_json::Map::new();
        let model_type = self.get_geokey_short(geokey::GT_MODEL_TYPE_GEO_KEY);
        info.insert(
            "modelType".into(),
            match model_type {
                Some(model_type::PROJECTED) => serde_json::Value::String("Projected".into()),
                Some(model_type::GEOGRAPHIC) => serde_json::Value::String("Geographic".into()),
                Some(model_type::GEOCENTRIC) => serde_json::Value::String("Geocentric".into()),
                _ => serde_json::Value::String("Unknown".into()),
            },
        );

        let geo_type = self.get_geokey_short(geokey::GEOGRAPHIC_TYPE_GEO_KEY);
        if let Some(code) = geo_type {
            info.insert("geographicTypeCode".into(), serde_json::json!(code));
            info.insert(
                "geographicType".into(),
                match code {
                    crs_code::WGS_84 => serde_json::Value::String("WGS 84".into()),
                    crs_code::WGS_72 => serde_json::Value::String("WGS 72".into()),
                    crs_code::NAD_83 => serde_json::Value::String("NAD 83".into()),
                    _ => serde_json::Value::String(format!("EPSG:{code}")),
                },
            );
        }

        let vcs = self.get_geokey_short(geokey::VERTICAL_CST_TYPE_GEO_KEY);
        if let Some(code) = vcs {
            info.insert("verticalCSTypeCode".into(), serde_json::json!(code));
        }

        serde_json::Value::Object(info).to_string()
    }

    /// Number of tiles (if tiled TIFF), otherwise 0.
    #[wasm_bindgen(getter)]
    pub fn tile_count(&self) -> u32 {
        if self.inner.is_tiled {
            self.inner.tile_offsets.len() as u32
        } else {
            0
        }
    }

    /// Number of strips.
    #[wasm_bindgen(js_name = "stripCount")]
    pub fn strip_count(&self) -> u32 {
        if self.inner.is_tiled {
            0
        } else {
            self.inner.strip_offsets.len() as u32
        }
    }
}

impl GeotiffInfo {
    /// Look up a GeoKey short value.
    fn get_geokey_short(&self, key_id: u16) -> Option<u16> {
        for (tag, _, _, val) in &self.inner.geo_keys {
            if *tag == key_id {
                if let StringOrShort::Short(v) = val {
                    return Some(*v);
                }
                break; // String values don't match
            }
        }
        None
    }
}

// ===========================================================================
// Quantized-Mesh Result
// ===========================================================================

/// Cesium quantized-mesh terrain tile encoded as binary.
#[wasm_bindgen]
pub struct QuantizedMeshResult {
    data: Vec<u8>,
}

#[wasm_bindgen]
impl QuantizedMeshResult {
    /// Raw quantized-mesh binary data as Uint8Array.
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> js_sys::Uint8Array {
        let arr = js_sys::Uint8Array::new_with_length(self.data.len() as u32);
        arr.copy_from(&self.data);
        arr
    }

    /// Size of the encoded tile in bytes.
    #[wasm_bindgen(getter)]
    pub fn byte_length(&self) -> usize {
        self.data.len()
    }
}

/// Tileset result containing tileset.json and quantized-mesh tiles.
#[wasm_bindgen]
pub struct TerrainTilesetResult {
    tileset_json: String,
    tiles: Vec<Vec<u8>>,
    tile_uris: Vec<String>,
}

#[wasm_bindgen]
impl TerrainTilesetResult {
    /// The tileset.json content as a string.
    #[wasm_bindgen(getter, js_name = "tilesetJson")]
    pub fn tileset_json(&self) -> String {
        self.tileset_json.clone()
    }

    /// Get a specific tile's binary data by index.
    #[wasm_bindgen]
    pub fn tile(&self, index: usize) -> js_sys::Uint8Array {
        if index < self.tiles.len() {
            let arr = js_sys::Uint8Array::new_with_length(self.tiles[index].len() as u32);
            arr.copy_from(&self.tiles[index]);
            arr
        } else {
            js_sys::Uint8Array::new_with_length(0)
        }
    }

    /// Total number of tiles in the tileset.
    #[wasm_bindgen(getter)]
    pub fn tile_count(&self) -> u32 {
        self.tiles.len() as u32
    }

    /// Get the URI/filename of a tile by index.
    #[wasm_bindgen(js_name = "tileUri")]
    pub fn tile_uri(&self, index: usize) -> String {
        self.tile_uris.get(index).cloned().unwrap_or_default()
    }

    /// Total bytes across all tiles.
    #[wasm_bindgen(getter, js_name = "totalBytes")]
    pub fn total_bytes(&self) -> usize {
        self.tiles.iter().map(|t| t.len()).sum()
    }
}

// ===========================================================================
// TIFF Parsing — Core
// ===========================================================================

/// Parse a TIFF/GeoTIFF file from raw bytes.
pub fn parse_geotiff_core(bytes: &[u8]) -> Result<GeotiffInfo, String> {
    let limit = crate::get_current_input_limit();
    if limit > 0 && bytes.len() > limit {
        return Err(format!(
            "Input too large ({} > {} bytes): GeoTIFF",
            bytes.len(),
            limit
        ));
    }
    parse_geotiff_impl(bytes)
}

fn parse_geotiff_impl(bytes: &[u8]) -> Result<GeotiffInfo, String> {
    if bytes.len() < 8 {
        return Err("GeoTIFF: file too small (less than 8 bytes)".into());
    }

    // 1. Parse header
    let byte_order = match &bytes[0..2] {
        b"II" => ByteOrder::LittleEndian,
        b"MM" => ByteOrder::BigEndian,
        _ => {
            return Err(format!(
                "GeoTIFF: invalid byte order marker: {:02X}{:02X}",
                bytes[0], bytes[1]
            ))
        }
    };

    let magic = read_u16(bytes, 2, byte_order)?;
    if magic != 42 {
        return Err(format!(
            "GeoTIFF: invalid magic number (expected 42, got {magic})"
        ));
    }

    let first_ifd_offset = read_u32(bytes, 4, byte_order)? as usize;

    // 2. Parse IFDs
    let mut entries: Vec<IfdEntry> = Vec::new();
    let ifd_offset = first_ifd_offset;

    // Parse first IFD (we don't chase chained IFDs for typical single-image GeoTIFF)
    if ifd_offset + 2 > bytes.len() {
        return Err("GeoTIFF: IFD offset out of bounds".into());
    }

    let num_entries = read_u16(bytes, ifd_offset, byte_order)? as usize;
    let mut pos = ifd_offset + 2;

    for _ in 0..num_entries {
        if pos + 12 > bytes.len() {
            return Err("GeoTIFF: IFD entry extends beyond file".into());
        }
        let entry = IfdEntry {
            tag_id: read_u16(bytes, pos, byte_order)?,
            field_type: read_u16(bytes, pos + 2, byte_order)?,
            count: read_u32(bytes, pos + 4, byte_order)?,
            value_bytes_offset: pos + 8,
            value_u32: read_u32(bytes, pos + 8, byte_order)?,
        };
        entries.push(entry);
        pos += 12;
    }

    // 3. Extract values from IFD entries
    let mut image_width: u32 = 0;
    let mut image_length: u32 = 0;
    let mut bits_per_sample: u16 = 8;
    let mut compression: u16 = compression::NONE;
    let mut sample_format: u16 = 1; // default: unsigned integer
    let mut strip_offsets: Vec<u32> = Vec::new();
    let mut strip_byte_counts: Vec<u64> = Vec::new();
    let mut rows_per_strip: u32 = 0xFFFFFFFF; // default: entire image
    let mut tile_width: u32 = 0;
    let mut tile_length: u32 = 0;
    let mut tile_offsets: Vec<u64> = Vec::new();
    let mut tile_byte_counts: Vec<u64> = Vec::new();
    let mut geo_key_dir_data: Option<Vec<u16>> = None;
    let mut geo_double_params: Vec<f64> = Vec::new();
    let mut geo_ascii_params: String = String::new();

    for entry in &entries {
        match entry.tag_id {
            tag::IMAGE_WIDTH => image_width = read_tag_value_u32(bytes, entry, byte_order)?,
            tag::IMAGE_LENGTH => image_length = read_tag_value_u32(bytes, entry, byte_order)?,
            tag::BITS_PER_SAMPLE => bits_per_sample = read_tag_value_u16(bytes, entry, byte_order)?,
            tag::COMPRESSION => compression = read_tag_value_u16(bytes, entry, byte_order)?,
            tag::SAMPLE_FORMAT => sample_format = read_tag_value_u16(bytes, entry, byte_order)?,
            tag::STRIP_OFFSETS => strip_offsets = read_tag_values_u32(bytes, entry, byte_order)?,
            tag::STRIP_BYTE_COUNTS => {
                strip_byte_counts = read_tag_values_u64(bytes, entry, byte_order)?
            }
            tag::ROWS_PER_STRIP => rows_per_strip = read_tag_value_u32(bytes, entry, byte_order)?,
            tag::TILE_WIDTH => tile_width = read_tag_value_u32(bytes, entry, byte_order)?,
            tag::TILE_LENGTH => tile_length = read_tag_value_u32(bytes, entry, byte_order)?,
            tag::TILE_OFFSETS => tile_offsets = read_tag_values_u64(bytes, entry, byte_order)?,
            tag::TILE_BYTE_COUNTS => {
                tile_byte_counts = read_tag_values_u64(bytes, entry, byte_order)?
            }
            tag::GEO_KEY_DIRECTORY => {
                geo_key_dir_data = Some(read_tag_values_u16(bytes, entry, byte_order)?);
            }
            tag::GEO_DOUBLE_PARAMS => {
                geo_double_params = read_tag_values_f64(bytes, entry, byte_order)?;
            }
            tag::GEO_ASCII_PARAMS => {
                geo_ascii_params = read_tag_value_string(bytes, entry)?;
            }
            _ => {} // Ignore unknown tags
        }
    }

    if image_width == 0 || image_length == 0 {
        return Err("GeoTIFF: missing ImageWidth or ImageLength".into());
    }

    let is_tiled = !tile_offsets.is_empty();

    // 4. Parse GeoKeys
    let geo_keys = if let Some(dir) = geo_key_dir_data {
        parse_geo_keys(&dir, &geo_ascii_params)
    } else {
        Vec::new()
    };

    let inner = GeotiffInternal {
        byte_order,
        image_width,
        image_length,
        bits_per_sample,
        compression,
        sample_format,
        strip_offsets,
        strip_byte_counts,
        rows_per_strip,
        tile_width,
        tile_length,
        tile_offsets,
        tile_byte_counts,
        geo_keys,
        geo_double_params,
        geo_ascii_params,
        is_tiled,
    };

    // 5. Decode elevation data
    let elevations = if is_tiled {
        decode_tiled_data(&inner, bytes)?
    } else {
        decode_strip_data(&inner, bytes)?
    };

    // 6. Compute bounds from GeoKeys + image dimensions
    let bounds = compute_bounds(&inner);

    Ok(GeotiffInfo {
        inner,
        elevations,
        bounds,
    })
}

// ===========================================================================
// TIFF Value Reading Helpers
// ===========================================================================

fn read_u16(bytes: &[u8], offset: usize, order: ByteOrder) -> Result<u16, String> {
    if offset + 2 > bytes.len() {
        return Err("read_u16: out of bounds".into());
    }
    Ok(match order {
        ByteOrder::LittleEndian => u16::from_le_bytes([bytes[offset], bytes[offset + 1]]),
        ByteOrder::BigEndian => u16::from_be_bytes([bytes[offset], bytes[offset + 1]]),
    })
}

fn read_u32(bytes: &[u8], offset: usize, order: ByteOrder) -> Result<u32, String> {
    if offset + 4 > bytes.len() {
        return Err("read_u32: out of bounds".into());
    }
    Ok(match order {
        ByteOrder::LittleEndian => u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]),
        ByteOrder::BigEndian => u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]),
    })
}

fn read_f32(bytes: &[u8], offset: usize, order: ByteOrder) -> Result<f32, String> {
    let bits = read_u32(bytes, offset, order)?;
    Ok(f32::from_bits(bits))
}

fn read_f64(bytes: &[u8], offset: usize, order: ByteOrder) -> Result<f64, String> {
    if offset + 8 > bytes.len() {
        return Err("read_f64: out of bounds".into());
    }
    Ok(match order {
        ByteOrder::LittleEndian => f64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]),
        ByteOrder::BigEndian => f64::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]),
    })
}

/// Read a single u16 value from a tag entry.
fn read_tag_value_u16(bytes: &[u8], entry: &IfdEntry, order: ByteOrder) -> Result<u16, String> {
    let type_size = field_type_size(entry.field_type);
    let total_size = type_size * entry.count as usize;
    if total_size <= 4 {
        // Value is inline in the 4-byte value/offset field
        read_u16(bytes, entry.value_bytes_offset, order)
    } else {
        // Value is at the offset stored in the 4-byte field
        read_u16(bytes, entry.value_u32 as usize, order)
    }
}

/// Read a single u32 value from a tag entry.
fn read_tag_value_u32(bytes: &[u8], entry: &IfdEntry, order: ByteOrder) -> Result<u32, String> {
    let type_size = field_type_size(entry.field_type);
    let total_size = type_size * entry.count as usize;
    if total_size <= 4 {
        // Value is inline
        read_u32(bytes, entry.value_bytes_offset, order)
    } else {
        // Value is at offset
        read_u32(bytes, entry.value_u32 as usize, order)
    }
}

/// Read multiple u32 values from a tag entry.
fn read_tag_values_u32(
    bytes: &[u8],
    entry: &IfdEntry,
    order: ByteOrder,
) -> Result<Vec<u32>, String> {
    let type_size = field_type_size(entry.field_type);
    let total_size = type_size * entry.count as usize;
    let mut result = Vec::with_capacity(entry.count as usize);

    if total_size <= 4 {
        // Values stored inline in the 4-byte value field
        let base = entry.value_bytes_offset;
        for i in 0..entry.count as usize {
            let off = base + i * type_size;
            result.push(read_u32(bytes, off, order)?);
        }
    } else {
        // Values at offset
        let mut base = entry.value_u32 as usize;
        for _ in 0..entry.count as usize {
            result.push(read_u32(bytes, base, order)?);
            base += type_size;
        }
    }

    Ok(result)
}

/// Read multiple u64 values from a tag entry (handles LONG/BIGTIFF-like scenarios and inline SHORT/LONG).
fn read_tag_values_u64(
    bytes: &[u8],
    entry: &IfdEntry,
    order: ByteOrder,
) -> Result<Vec<u64>, String> {
    let type_size = field_type_size(entry.field_type);
    let total_size = type_size * entry.count as usize;
    let mut result = Vec::with_capacity(entry.count as usize);

    if total_size <= 4 {
        // Inline
        let base = entry.value_bytes_offset;
        for i in 0..entry.count as usize {
            let off = base + i * type_size;
            match entry.field_type {
                field_type::SHORT | field_type::SSHORT => {
                    result.push(read_u16(bytes, off, order)? as u64);
                }
                field_type::LONG | field_type::SLONG => {
                    result.push(read_u32(bytes, off, order)? as u64);
                }
                field_type::FLOAT => {
                    result.push(read_f32(bytes, off, order)?.to_bits() as u64);
                }
                _ => result.push(read_u32(bytes, off, order)? as u64),
            }
        }
    } else {
        // At offset
        let mut base = entry.value_u32 as usize;
        for _ in 0..entry.count as usize {
            match entry.field_type {
                field_type::SHORT | field_type::SSHORT => {
                    result.push(read_u16(bytes, base, order)? as u64);
                    base += 2;
                }
                field_type::LONG | field_type::SLONG => {
                    result.push(read_u32(bytes, base, order)? as u64);
                    base += 4;
                }
                field_type::DOUBLE => {
                    result.push(read_f64(bytes, base, order)?.to_bits());
                    base += 8;
                }
                _ => {
                    result.push(read_u32(bytes, base, order)? as u64);
                    base += type_size.max(1);
                }
            }
        }
    }

    Ok(result)
}

/// Read multiple u16 values from a tag entry.
fn read_tag_values_u16(
    bytes: &[u8],
    entry: &IfdEntry,
    order: ByteOrder,
) -> Result<Vec<u16>, String> {
    let type_size = field_type_size(entry.field_type);
    let total_size = type_size * entry.count as usize;
    let mut result = Vec::with_capacity(entry.count as usize);

    if total_size <= 4 {
        let base = entry.value_bytes_offset;
        for i in 0..entry.count as usize {
            let off = base + i * type_size;
            result.push(read_u16(bytes, off, order)?);
        }
    } else {
        let mut base = entry.value_u32 as usize;
        for _ in 0..entry.count as usize {
            result.push(read_u16(bytes, base, order)?);
            base += type_size.max(1);
        }
    }

    Ok(result)
}

/// Read multiple f64 values from a tag entry.
fn read_tag_values_f64(
    bytes: &[u8],
    entry: &IfdEntry,
    order: ByteOrder,
) -> Result<Vec<f64>, String> {
    let count = entry.count as usize;
    let type_size = field_type_size(entry.field_type);
    let total_size = type_size * count;

    if total_size <= 4 {
        // Unusual but handle: very small double array inline
        let base = entry.value_bytes_offset;
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            match entry.field_type {
                field_type::DOUBLE => {
                    result.push(0.0);
                }
                _ => {
                    result.push(read_f32(bytes, base + i * type_size, order)? as f64);
                }
            }
        }
        Ok(result)
    } else {
        let base = entry.value_u32 as usize;
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            match entry.field_type {
                field_type::DOUBLE => {
                    result.push(read_f64(bytes, base + i * 8, order)?);
                }
                _ => {
                    result.push(read_f32(bytes, base + i * type_size, order)? as f64);
                }
            }
        }
        Ok(result)
    }
}

/// Read a string value from a tag entry (ASCII type).
fn read_tag_value_string(bytes: &[u8], entry: &IfdEntry) -> Result<String, String> {
    let type_size = field_type_size(entry.field_type);
    let total_size = type_size * entry.count as usize;

    let base = if total_size <= 4 {
        entry.value_bytes_offset
    } else {
        entry.value_u32 as usize
    };
    let end = base + entry.count as usize;
    if end > bytes.len() {
        return Err("GeoTIFF: string value out of bounds".into());
    }
    String::from_utf8(bytes[base..end].to_vec())
        .map(|s| s.trim_end_matches(['\0', '|']).to_string())
        .map_err(|e| format!("GeoTIFF: invalid ASCII in tag: {e}"))
}

// ===========================================================================
// GeoKey Parsing
// ===========================================================================

/// Parse GeoKeyDirectory into a list of (key_id, tiff_tag_location, count, value).
fn parse_geo_keys(dir: &[u16], ascii_params: &str) -> Vec<(u16, u16, u16, StringOrShort)> {
    let mut keys = Vec::new();
    // GeoKeyDirectory: [version, revision, minor_revision, count, ...key entries]
    // Each key entry: [key_id, tiff_tag_location, count, value/offset]
    if dir.len() < 4 {
        return keys;
    }

    let num_keys = dir[3] as usize;
    for i in 0..num_keys {
        let base = 4 + i * 4;
        if base + 4 > dir.len() {
            break;
        }
        let key_id = dir[base];
        let tiff_tag_loc = dir[base + 1];
        let count = dir[base + 2];
        let value_or_offset = dir[base + 3];

        if tiff_tag_loc == 0 {
            // Value is stored directly
            keys.push((
                key_id,
                tiff_tag_loc,
                count,
                StringOrShort::Short(value_or_offset),
            ));
        } else if tiff_tag_loc == 34737 {
            // String value in GeoAsciiParams
            let offset = value_or_offset as usize;
            let len = count as usize;
            let _s: String = ascii_params
                .chars()
                .skip(offset)
                .take(len)
                .collect::<String>()
                .trim_end_matches(['\0', '|'])
                .to_string();
            keys.push((
                key_id,
                tiff_tag_loc,
                count,
                StringOrShort::AsciiOffset(offset, len),
            ));
        }
        // tiff_tag_loc == 34736 means value is in GeoDoubleParams (not commonly used for short keys)
    }
    keys
}

// ===========================================================================
// Data Decoding
// ===========================================================================

/// Decode strip-organized image data into f32 elevation values.
fn decode_strip_data(info: &GeotiffInternal, bytes: &[u8]) -> Result<Vec<f32>, String> {
    match info.bits_per_sample {
        32 => decode_strip_f32(info, bytes), // Assume float for 32-bit (common in GeoTIFF elevation)
        16 => decode_strip_u16_to_f32(info, bytes),
        8 => decode_strip_u8_to_f32(info, bytes),
        _ => Err(format!(
            "GeoTIFF: unsupported bit depth: {} bits (sample_format={})",
            info.bits_per_sample, info.sample_format
        )),
    }
}

/// Decode float32 strip data.
fn decode_strip_f32(info: &GeotiffInternal, bytes: &[u8]) -> Result<Vec<f32>, String> {
    let total_pixels = info.image_width as usize * info.image_length as usize;
    let mut elevations = Vec::with_capacity(total_pixels);
    let w = info.image_width as usize;

    for strip_idx in 0..info.strip_offsets.len() {
        let offset = info.strip_offsets[strip_idx] as usize;
        let byte_count = info.strip_byte_counts[strip_idx] as usize;
        if offset + byte_count > bytes.len() {
            return Err(format!(
                "GeoTIFF: strip {} data extends beyond file (offset={}, count={}, file_len={})",
                strip_idx,
                offset,
                byte_count,
                bytes.len()
            ));
        }

        let raw_data = &bytes[offset..offset + byte_count];
        let decompressed = decompress_data(raw_data, info.compression)?;

        let rows_this_strip = info
            .rows_per_strip
            .min((info.image_length - (strip_idx as u32 * info.rows_per_strip)).max(1))
            as usize;
        let pixels_this_strip = rows_this_strip * w;
        let expected_bytes = pixels_this_strip * 4;

        if decompressed.len() < expected_bytes {
            return Err(format!(
                "GeoTIFF: strip {} decompressed data too small (got {} bytes, expected {})",
                strip_idx,
                decompressed.len(),
                expected_bytes
            ));
        }

        for i in 0..pixels_this_strip {
            let off = i * 4;
            let val = f32::from_le_bytes([
                decompressed[off],
                decompressed[off + 1],
                decompressed[off + 2],
                decompressed[off + 3],
            ]);
            elevations.push(val);
        }
    }

    Ok(elevations)
}

/// Decode uint16 strip data → f32.
fn decode_strip_u16_to_f32(info: &GeotiffInternal, bytes: &[u8]) -> Result<Vec<f32>, String> {
    let total_pixels = info.image_width as usize * info.image_length as usize;
    let mut elevations = Vec::with_capacity(total_pixels);
    let w = info.image_width as usize;

    for strip_idx in 0..info.strip_offsets.len() {
        let offset = info.strip_offsets[strip_idx] as usize;
        let byte_count = info.strip_byte_counts[strip_idx] as usize;
        if offset + byte_count > bytes.len() {
            return Err("GeoTIFF: strip data extends beyond file".into());
        }

        let raw_data = &bytes[offset..offset + byte_count];
        let decompressed = decompress_data(raw_data, info.compression)?;

        let rows_this_strip = info.rows_this_strip(strip_idx);
        let pixels_this_strip = rows_this_strip * w;
        let expected_bytes = pixels_this_strip * 2;

        if decompressed.len() < expected_bytes {
            return Err("GeoTIFF: strip decompressed data too small for u16".into());
        }

        for i in 0..pixels_this_strip {
            let off = i * 2;
            let val = u16::from_le_bytes([decompressed[off], decompressed[off + 1]]);
            elevations.push(val as f32);
        }
    }

    Ok(elevations)
}

/// Decode uint8 strip data → f32.
fn decode_strip_u8_to_f32(info: &GeotiffInternal, bytes: &[u8]) -> Result<Vec<f32>, String> {
    let total_pixels = info.image_width as usize * info.image_length as usize;
    let mut elevations = Vec::with_capacity(total_pixels);
    let w = info.image_width as usize;

    for strip_idx in 0..info.strip_offsets.len() {
        let offset = info.strip_offsets[strip_idx] as usize;
        let byte_count = info.strip_byte_counts[strip_idx] as usize;
        if offset + byte_count > bytes.len() {
            return Err("GeoTIFF: strip data extends beyond file".into());
        }

        let raw_data = &bytes[offset..offset + byte_count];
        let decompressed = decompress_data(raw_data, info.compression)?;

        let rows_this_strip = info.rows_this_strip(strip_idx);
        let pixels_this_strip = rows_this_strip * w;
        let expected_bytes = pixels_this_strip;

        if decompressed.len() < expected_bytes {
            return Err("GeoTIFF: strip decompressed data too small for u8".into());
        }

        for &b in &decompressed[..pixels_this_strip] {
            elevations.push(b as f32);
        }
    }

    Ok(elevations)
}

/// Decode tile-organized image data into f32 elevation values.
fn decode_tiled_data(info: &GeotiffInternal, bytes: &[u8]) -> Result<Vec<f32>, String> {
    if info.tile_width == 0 || info.tile_length == 0 {
        return Err("GeoTIFF: tiled image but tile dimensions are 0".into());
    }

    let total_pixels = info.image_width as usize * info.image_length as usize;
    let mut elevations = vec![0.0f32; total_pixels]; // Initialize to 0 for partial tiles

    let tw = info.tile_width as usize;
    let tl = info.tile_length as usize;
    let iw = info.image_width as usize;
    let ih = info.image_length as usize;

    // Number of tiles in each direction
    let tiles_across = iw.div_ceil(tw);
    let tiles_down = ih.div_ceil(tl);

    if info.tile_offsets.len() < tiles_across * tiles_down {
        return Err(format!(
            "GeoTIFF: too few tile offsets (expected {}, got {})",
            tiles_across * tiles_down,
            info.tile_offsets.len()
        ));
    }

    for ty in 0..tiles_down {
        for tx in 0..tiles_across {
            let tile_idx = ty * tiles_across + tx;
            if tile_idx >= info.tile_offsets.len() {
                break;
            }

            let offset = info.tile_offsets[tile_idx] as usize;
            let byte_count = info.tile_byte_counts[tile_idx] as usize;
            if offset + byte_count > bytes.len() {
                return Err(format!(
                    "GeoTIFF: tile {} data extends beyond file",
                    tile_idx
                ));
            }

            let raw_data = &bytes[offset..offset + byte_count];
            let decompressed = decompress_data(raw_data, info.compression)?;

            let pixels_in_tile = tw * tl;
            let bytes_per_pixel = info.bits_per_sample as usize / 8;
            let _expected_bytes = pixels_in_tile * bytes_per_pixel;

            // Fill pixels for this tile
            let col_start = tx * tw;
            let row_start = ty * tl;
            let col_end = (col_start + tw).min(iw);
            let row_end = (row_start + tl).min(ih);

            for local_y in 0..row_end - row_start {
                for local_x in 0..col_end - col_start {
                    let pix_idx = local_y * tw + local_x;
                    let global_x = col_start + local_x;
                    let global_y = row_start + local_y;
                    let global_idx = global_y * iw + global_x;

                    let byte_off = pix_idx * bytes_per_pixel;
                    if byte_off + bytes_per_pixel <= decompressed.len() {
                        let val = match info.bits_per_sample {
                            32 => f32::from_le_bytes([
                                decompressed[byte_off],
                                decompressed[byte_off + 1],
                                decompressed[byte_off + 2],
                                decompressed[byte_off + 3],
                            ]),
                            16 => u16::from_le_bytes([
                                decompressed[byte_off],
                                decompressed[byte_off + 1],
                            ]) as f32,
                            8 => decompressed[byte_off] as f32,
                            _ => 0.0,
                        };
                        elevations[global_idx] = val;
                    }
                }
            }
        }
    }

    Ok(elevations)
}

/// Decompress data based on compression method.
fn decompress_data(data: &[u8], compression: u16) -> Result<Vec<u8>, String> {
    match compression {
        compression::NONE => Ok(data.to_vec()),
        compression::DEFLATE | compression::DEFLATE_GEOTIFF => {
            let mut decoder = ZlibDecoder::new(data);
            let mut output = Vec::new();
            decoder
                .read_to_end(&mut output)
                .map_err(|e| format!("GeoTIFF: DEFLATE decompression failed: {e}"))?;
            Ok(output)
        }
        compression::LZW => Err("GeoTIFF: LZW compression is not yet supported (TODO)".into()),
        _ => Err(format!(
            "GeoTIFF: unsupported compression method: {compression}"
        )),
    }
}

impl GeotiffInternal {
    /// Compute the number of rows in a specific strip.
    fn rows_this_strip(&self, strip_idx: usize) -> usize {
        if self.rows_per_strip == 0xFFFFFFFF || self.rows_per_strip == 0 {
            return self.image_length as usize;
        }
        let rps = self.rows_per_strip as usize;
        let rows_so_far = strip_idx * rps;
        let remaining = self.image_length as usize - rows_so_far;
        rps.min(remaining)
    }
}

// ===========================================================================
// Bounds Computation
// ===========================================================================

/// Compute geographic bounds from GeoTIFF metadata.
/// Falls back to [0, 0, 1, 1] when no tie points or GeoTransform are available.
fn compute_bounds(_info: &GeotiffInternal) -> [f64; 4] {
    // Try to extract bounds from GeoDoubleParams (ModelTiepointTag + ModelPixelScaleTag)
    // For simplicity, we check if there's a GeoTransform-like setup in the GeoDoubleParams.
    // In a real GeoTIFF, bounds come from ModelTiepointTag (33922) and ModelPixelScaleTag (33550).
    // These are standard TIFF tags, not GeoKeys, but commonly present in GeoTIFF files.

    // If we had ModelTiepointTag + ModelPixelScaleTag, we'd compute:
    // origin_lon = tiepoint[3] - (tiepoint[0] * scale[0])  // upper-left
    // origin_lat = tiepoint[4] + (tiepoint[1] * scale[1])
    // max_lon = origin_lon + width * scale[0]
    // max_lat = origin_lat + height * scale[1]

    // For now, return a placeholder. Real GeoTIFFs would use the metadata.
    // This will be improved as we parse additional TIFF tags.
    [0.0, 0.0, 1.0, 1.0]
}

// ===========================================================================
// Quantized-Mesh Encoding
// ===========================================================================

/// Encode a height matrix into a Cesium quantized-mesh binary format.
///
/// # Arguments
/// * `heights` — flat f32 array, row-major (width × height)
/// * `width` — number of columns
/// * `height` — number of rows
/// * `bounds` — geographic bounds [min_lng, min_lat, max_lng, max_lat]
/// * `center` — tile center [x, y, z] in ECEF
///
/// # Returns
/// Binary quantized-mesh data.
pub fn encode_quantized_mesh_core(
    heights: &[f32],
    width: u32,
    height: u32,
    _bounds: &[f64; 4],
    center: &[f64; 3],
) -> Result<Vec<u8>, String> {
    if width < 2 || height < 2 {
        return Err("QuantizedMesh: grid must be at least 2×2".into());
    }
    if heights.len() != (width * height) as usize {
        return Err(format!(
            "QuantizedMesh: heights length {} != width×height {}",
            heights.len(),
            width * height
        ));
    }

    // Compute height range
    let mut min_h = f32::INFINITY;
    let mut max_h = f32::NEG_INFINITY;
    for &h in heights {
        min_h = min_h.min(h);
        max_h = max_h.max(h);
    }
    let h_range = max_h - min_h;
    if h_range == 0.0 {
        // Flat terrain
    }

    let vertex_count = width * height;
    let use_32bit_indices = vertex_count > 65535;

    // Build header
    let mut buf: Vec<u8> = Vec::with_capacity(128 + vertex_count as usize * 6);
    // Center X (encoded as f64 → bytes)
    buf.extend_from_slice(&center[0].to_le_bytes());
    // Center Y
    buf.extend_from_slice(&center[1].to_le_bytes());
    // Center Z
    buf.extend_from_slice(&center[2].to_le_bytes());
    // Minimum height (f16 — approximate with f32 truncated to 2 bytes)
    buf.extend_from_slice(&min_h.to_le_bytes()[..2]); // Low 2 bytes as f16 approx
                                                      // Maximum height
    buf.extend_from_slice(&max_h.to_le_bytes()[..2]);
    // Oct-encoded normal (unused, set to 0)
    buf.push(0);
    // Water mask (unused, set to 0)
    buf.push(0);
    // Header byte size (variable) — we'll patch this later
    let header_size_offset = buf.len();
    buf.extend_from_slice(&88u32.to_le_bytes()); // placeholder

    // Vertex data
    let u16_max = 65535u16;
    buf.extend_from_slice(&vertex_count.to_le_bytes()); // vertex count

    // Quantized coordinates
    for row in 0..height {
        for col in 0..width {
            // u: quantized longitude (0..65535)
            let u_val = if width > 1 {
                ((col as f64 / (width - 1) as f64) * u16_max as f64) as u16
            } else {
                0
            };
            buf.extend_from_slice(&u_val.to_le_bytes());

            // v: quantized latitude (0..65535, note: inverted — south=0, north=65535)
            let v_val = if height > 1 {
                ((row as f64 / (height - 1) as f64) * u16_max as f64) as u16
            } else {
                0
            };
            buf.extend_from_slice(&v_val.to_le_bytes());

            // height: quantized Z
            let h_idx = (row * width + col) as usize;
            let h_val = heights[h_idx];
            let h_quantized = if h_range > 0.0 {
                ((h_val - min_h) / h_range * u16_max as f32) as u16
            } else {
                0 // Flat terrain
            };
            buf.extend_from_slice(&h_quantized.to_le_bytes());
        }
    }

    // Index data — triangulate regular grid
    let triangle_count = (width - 1) * (height - 1) * 2;
    buf.extend_from_slice(&triangle_count.to_le_bytes());

    // Triangle indices
    if use_32bit_indices {
        for row in 0..height - 1 {
            for col in 0..width - 1 {
                let i0 = row * width + col;
                let i1 = i0 + 1;
                let i2 = (row + 1) * width + col;
                let i3 = i2 + 1;
                // Triangle 1: i0, i2, i1
                buf.extend_from_slice(&i0.to_le_bytes());
                buf.extend_from_slice(&i2.to_le_bytes());
                buf.extend_from_slice(&i1.to_le_bytes());
                // Triangle 2: i1, i2, i3
                buf.extend_from_slice(&i1.to_le_bytes());
                buf.extend_from_slice(&i2.to_le_bytes());
                buf.extend_from_slice(&i3.to_le_bytes());
            }
        }
    } else {
        for row in 0..height - 1 {
            for col in 0..width - 1 {
                let i0 = (row * width + col) as u16;
                let i1 = i0 + 1;
                let i2 = ((row + 1) * width + col) as u16;
                let i3 = i2 + 1;
                buf.extend_from_slice(&i0.to_le_bytes());
                buf.extend_from_slice(&i2.to_le_bytes());
                buf.extend_from_slice(&i1.to_le_bytes());
                buf.extend_from_slice(&i1.to_le_bytes());
                buf.extend_from_slice(&i2.to_le_bytes());
                buf.extend_from_slice(&i3.to_le_bytes());
            }
        }
    }

    // Edge indices — west, south, east, north borders
    // West edge (col=0): rows 0..height
    let west_count = height as usize;
    buf.extend_from_slice(&(west_count as u32).to_le_bytes());
    for row in 0..height {
        let idx = row * width;
        if use_32bit_indices {
            buf.extend_from_slice(&idx.to_le_bytes());
        } else {
            buf.extend_from_slice(&(idx as u16).to_le_bytes());
        }
    }

    // South edge (row=height-1): cols 0..width
    let south_count = width as usize;
    buf.extend_from_slice(&(south_count as u32).to_le_bytes());
    for col in 0..width {
        let idx = (height - 1) * width + col;
        if use_32bit_indices {
            buf.extend_from_slice(&idx.to_le_bytes());
        } else {
            buf.extend_from_slice(&(idx as u16).to_le_bytes());
        }
    }

    // East edge (col=width-1): rows 0..height (reversed for proper winding)
    let east_count = height as usize;
    buf.extend_from_slice(&(east_count as u32).to_le_bytes());
    for row in (0..height).rev() {
        let idx = row * width + width - 1;
        if use_32bit_indices {
            buf.extend_from_slice(&idx.to_le_bytes());
        } else {
            buf.extend_from_slice(&(idx as u16).to_le_bytes());
        }
    }

    // North edge (row=0): cols 0..width (reversed for proper winding)
    let north_count = width as usize;
    buf.extend_from_slice(&(north_count as u32).to_le_bytes());
    for col in (0..width).rev() {
        let idx = col;
        if use_32bit_indices {
            buf.extend_from_slice(&idx.to_le_bytes());
        } else {
            buf.extend_from_slice(&(idx as u16).to_le_bytes());
        }
    }

    // Patch header size
    let actual_header_size = header_size_offset + 4; // 4 bytes for the header_size field itself
    let header_bytes = actual_header_size.to_le_bytes();
    buf[header_size_offset] = header_bytes[0];
    buf[header_size_offset + 1] = header_bytes[1];
    buf[header_size_offset + 2] = header_bytes[2];
    buf[header_size_offset + 3] = header_bytes[3];

    Ok(buf)
}

// ===========================================================================
// Terrain Tileset Generation
// ===========================================================================

/// Generate a 3D Tiles terrain tileset with quantized-mesh tiles.
///
/// Creates a tile pyramid with multiple LOD levels, each level downsampling
/// the elevation data by 2x.
///
/// # Arguments
/// * `heights` — flat f32 array, row-major (width × height)
/// * `width` — number of columns
/// * `height` — number of rows
/// * `bounds` — geographic bounds [min_lng, min_lat, max_lng, max_lat]
/// * `center` — tile center [x, y, z] in ECEF
/// * `max_zoom` — maximum zoom level (default: 4)
///
/// # Returns
/// A TerrainTilesetResult containing tileset.json and tile binary data.
pub fn encode_terrain_tileset_core(
    heights: &[f32],
    width: u32,
    height: u32,
    bounds: &[f64; 4],
    center: &[f64; 3],
    max_zoom: u32,
) -> Result<TerrainTilesetResult, String> {
    if width < 2 || height < 2 {
        return Err("TerrainTileset: grid must be at least 2×2".into());
    }
    if heights.len() != (width * height) as usize {
        return Err("TerrainTileset: heights length mismatch".into());
    }

    let mut tiles = Vec::new();
    let mut tile_uris = Vec::new();
    let zoom = max_zoom.min(6); // Cap zoom to avoid excessive tiles

    // Generate tiles at each zoom level
    let mut current_heights = heights.to_vec();
    let mut current_w = width;
    let mut current_h = height;

    for level in 0..=zoom {
        // Encode current level as a quantized-mesh tile
        let mesh_data =
            encode_quantized_mesh_core(&current_heights, current_w, current_h, bounds, center)?;

        let uri = format!("terrain_{level}.cmpt");
        tiles.push(mesh_data);
        tile_uris.push(uri);

        // Downsample for next level
        if level < zoom && current_w > 2 && current_h > 2 {
            let next_w = (current_w / 2).max(2);
            let next_h = (current_h / 2).max(2);
            let mut next_heights = Vec::with_capacity((next_w * next_h) as usize);
            for y in 0..next_h {
                for x in 0..next_w {
                    let sx = (x * 2) as usize;
                    let sy = (y * 2) as usize;
                    let val = current_heights[sy * current_w as usize + sx];
                    next_heights.push(val);
                }
            }
            current_heights = next_heights;
            current_w = next_w;
            current_h = next_h;
        }
    }

    // Build tileset.json
    let tileset = build_tileset_json(bounds, zoom, center);

    Ok(TerrainTilesetResult {
        tileset_json: tileset,
        tiles,
        tile_uris,
    })
}

/// Build a tileset.json structure for terrain tiles.
fn build_tileset_json(bounds: &[f64; 4], max_zoom: u32, center: &[f64; 3]) -> String {
    // Build a simple tree: root → children at each zoom level
    // Using box boundingVolume for simplicity
    let min_lng = bounds[0];
    let min_lat = bounds[1];
    let max_lng = bounds[2];
    let max_lat = bounds[3];

    let mut children = Vec::new();
    for level in 0..=max_zoom {
        let geo_error = 1000.0 / (1 << level) as f64;
        children.push(serde_json::json!({
            "boundingVolume": {
                "box": [
                    center[0], center[1], center[2],
                    (max_lng - min_lng) * 111_320.0 / 2.0, 0.0, 0.0,
                    0.0, (max_lat - min_lat) * 111_320.0 / 2.0, 0.0,
                    0.0, 0.0, 100.0  // height approximation
                ]
            },
            "geometricError": geo_error,
            "refine": "ADD",
            "content": {
                "uri": format!("terrain_{}.cmpt", level)
            }
        }));
    }

    let tileset = serde_json::json!({
        "asset": {
            "version": "1.1",
            "tilesetVersion": "1.0.0"
        },
        "geometricError": 1000.0,
        "root": {
            "boundingVolume": {
                "box": [
                    center[0], center[1], center[2],
                    (max_lng - min_lng) * 111_320.0 / 2.0, 0.0, 0.0,
                    0.0, (max_lat - min_lat) * 111_320.0 / 2.0, 0.0,
                    0.0, 0.0, 100.0
                ]
            },
            "geometricError": 1000.0,
            "refine": "ADD",
            "children": children
        }
    });

    serde_json::to_string_pretty(&tileset).unwrap_or_default()
}

// ===========================================================================
// WASM Exports
// ===========================================================================

/// Parse a GeoTIFF file from raw bytes.
///
/// Returns a `GeotiffInfo` object with metadata and elevation data.
///
/// # Example (JS)
/// ```js
/// const info = core.parseGeotiff(tiffBytes);
/// console.log(info.width(), info.height());
/// const elevations = info.elevation(); // Float32Array
/// ```
#[wasm_bindgen(js_name = "parseGeotiff")]
pub fn parse_geotiff(bytes: &[u8]) -> Result<GeotiffInfo, JsValue> {
    parse_geotiff_core(bytes).map_err(|e| {
        let err = crate::errors::SpatialError::parse_error(e);
        JsValue::from(err)
    })
}

/// Parse a single tile from a tiled GeoTIFF.
///
/// Returns Float32Array of elevation values for the specified tile.
#[wasm_bindgen(js_name = "parseGeotiffTile")]
pub fn parse_geotiff_tile(
    bytes: &[u8],
    tile_index: usize,
) -> Result<js_sys::Float32Array, JsValue> {
    let info = parse_geotiff_core(bytes).map_err(|e| {
        let err = crate::errors::SpatialError::parse_error(e);
        JsValue::from(err)
    })?;

    if !info.inner.is_tiled {
        return Err(JsValue::from(crate::errors::SpatialError::invalid_input(
            "Not a tiled GeoTIFF",
        )));
    }

    if tile_index >= info.inner.tile_offsets.len() {
        return Err(JsValue::from(crate::errors::SpatialError::index_error(
            format!(
                "Tile index {} out of range (max {})",
                tile_index,
                info.inner.tile_offsets.len() - 1
            ),
        )));
    }

    // For tiled images, the elevation data is already decoded in the full grid.
    // Return the relevant portion based on tile position.
    let tw = info.inner.tile_width as usize;
    let tl = info.inner.tile_length as usize;
    let iw = info.inner.image_width as usize;
    let tiles_across = iw.div_ceil(tw);
    let tx = tile_index % tiles_across;
    let ty = tile_index / tiles_across;
    let col_start = tx * tw;
    let row_start = ty * tl;
    let col_end = (col_start + tw).min(iw);
    let row_end = (row_start + tl).min(info.inner.image_length as usize);

    let mut tile_elevations = Vec::new();
    for y in row_start..row_end {
        for x in col_start..col_end {
            tile_elevations.push(info.elevations[y * iw + x]);
        }
    }

    let arr = js_sys::Float32Array::new_with_length(tile_elevations.len() as u32);
    arr.copy_from(&tile_elevations);
    Ok(arr)
}

/// Encode a height matrix into a Cesium quantized-mesh terrain tile.
///
/// # Arguments
/// * `heights` — Float32Array, row-major (width × height)
/// * `width` — number of columns
/// * `height` — number of rows
/// * `bounds` — Float64Array [min_lng, min_lat, max_lng, max_lat]
/// * `center` — Float64Array [x, y, z] in ECEF
///
/// # Returns
/// `QuantizedMeshResult` with the binary data.
#[wasm_bindgen(js_name = "encodeQuantizedMesh")]
pub fn encode_quantized_mesh(
    heights: &[f32],
    width: u32,
    height: u32,
    bounds: &[f64],
    center: &[f64],
) -> Result<QuantizedMeshResult, JsValue> {
    let bounds_arr: [f64; 4] = bounds
        .try_into()
        .map_err(|_| crate::errors::SpatialError::invalid_input("bounds must have 4 elements"))
        .map_err(JsValue::from)?;
    let center_arr: [f64; 3] = center
        .try_into()
        .map_err(|_| crate::errors::SpatialError::invalid_input("center must have 3 elements"))
        .map_err(JsValue::from)?;

    encode_quantized_mesh_core(heights, width, height, &bounds_arr, &center_arr)
        .map(|data| QuantizedMeshResult { data })
        .map_err(|e| JsValue::from(crate::errors::SpatialError::geometry_error(e)))
}

/// Generate a 3D Tiles terrain tileset with quantized-mesh tiles.
///
/// # Arguments
/// * `heights` — Float32Array, row-major (width × height)
/// * `width` — number of columns
/// * `height` — number of rows
/// * `bounds` — Float64Array [min_lng, min_lat, max_lng, max_lat]
/// * `center` — Float64Array [x, y, z] in ECEF
/// * `max_zoom` — maximum zoom level (default: 4)
///
/// # Returns
/// `TerrainTilesetResult` containing tileset.json and tile data.
#[wasm_bindgen(js_name = "encodeTerrainTileset")]
pub fn encode_terrain_tileset(
    heights: &[f32],
    width: u32,
    height: u32,
    bounds: &[f64],
    center: &[f64],
    max_zoom: u32,
) -> Result<TerrainTilesetResult, JsValue> {
    let bounds_arr: [f64; 4] = bounds
        .try_into()
        .map_err(|_| crate::errors::SpatialError::invalid_input("bounds must have 4 elements"))
        .map_err(JsValue::from)?;
    let center_arr: [f64; 3] = center
        .try_into()
        .map_err(|_| crate::errors::SpatialError::invalid_input("center must have 3 elements"))
        .map_err(JsValue::from)?;

    encode_terrain_tileset_core(heights, width, height, &bounds_arr, &center_arr, max_zoom)
        .map_err(|e| JsValue::from(crate::errors::SpatialError::tile_error(e)))
}

/// Check if GeoTIFF support is available (always true).
#[wasm_bindgen(js_name = "supportsGeotiff")]
pub fn supports_geotiff() -> bool {
    true
}

/// Get GeoTIFF support status as a human-readable string.
#[wasm_bindgen(js_name = "geotiffStatus")]
pub fn geotiff_status() -> String {
    String::from(
        "GeoTIFF support: AVAILABLE. Hand-written TIFF parser + DEFLATE decompression. \
         Supports Float32/Uint16/Uint8 elevation grids, strip and tile layouts, \
         GeoKey metadata. Outputs Cesium quantized-mesh terrain tiles.",
    )
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Helper: build a minimal uncompressed TIFF with float32 data
    // -------------------------------------------------------------------------

    fn build_minimal_tiff_f32(width: u32, height: u32, values: &[f32]) -> Vec<u8> {
        assert_eq!(values.len(), (width * height) as usize);

        let mut buf = Vec::new();

        // TIFF Header (little-endian)
        buf.extend_from_slice(b"II"); // Little endian
        buf.extend_from_slice(&42u16.to_le_bytes()); // Magic
        let ifd_offset_pos = buf.len();
        buf.extend_from_slice(&0u32.to_le_bytes()); // Placeholder for IFD offset

        // IFD starts here
        let ifd_start = buf.len();
        // Patch header
        buf[ifd_offset_pos..ifd_offset_pos + 4].copy_from_slice(&(ifd_start as u32).to_le_bytes());

        // Number of IFD entries
        let num_entries: u16 = 7; // Width, Height, BitsPerSample, Compression, StripOffsets, StripByteCounts, RowsPerStrip
        buf.extend_from_slice(&num_entries.to_le_bytes());

        // Image data is a single strip at the end
        let ne = num_entries as usize;
        let strip_data_offset_pos = ifd_start + 2 + ne * 12 + 4; // +4 for next IFD offset
        let strip_data_offset = strip_data_offset_pos as u32;
        let strip_data_size = values.len() * 4; // f32

        // IFD entries (must be sorted by tag ID)
        // Tag 256: ImageWidth
        buf.extend_from_slice(&256u16.to_le_bytes()); // tag
        buf.extend_from_slice(&4u16.to_le_bytes()); // type: LONG
        buf.extend_from_slice(&1u32.to_le_bytes()); // count
        buf.extend_from_slice(&width.to_le_bytes()); // value (inline)

        // Tag 257: ImageLength
        buf.extend_from_slice(&257u16.to_le_bytes());
        buf.extend_from_slice(&4u16.to_le_bytes()); // LONG
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&height.to_le_bytes());

        // Tag 258: BitsPerSample
        buf.extend_from_slice(&258u16.to_le_bytes());
        buf.extend_from_slice(&3u16.to_le_bytes()); // SHORT
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&32u16.to_le_bytes()); // 32 bits
        buf.extend_from_slice(&0u16.to_le_bytes()); // padding

        // Tag 259: Compression (1 = none)
        buf.extend_from_slice(&259u16.to_le_bytes());
        buf.extend_from_slice(&3u16.to_le_bytes()); // SHORT
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes()); // No compression
        buf.extend_from_slice(&0u16.to_le_bytes()); // padding

        // Tag 273: StripOffsets
        buf.extend_from_slice(&273u16.to_le_bytes());
        buf.extend_from_slice(&4u16.to_le_bytes()); // LONG
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&strip_data_offset.to_le_bytes());

        // Tag 278: RowsPerStrip (all rows in one strip)
        buf.extend_from_slice(&278u16.to_le_bytes());
        buf.extend_from_slice(&4u16.to_le_bytes()); // LONG
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&height.to_le_bytes());

        // Tag 279: StripByteCounts
        buf.extend_from_slice(&279u16.to_le_bytes());
        buf.extend_from_slice(&4u16.to_le_bytes()); // LONG
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&(strip_data_size as u32).to_le_bytes());

        // Next IFD offset (0 = no more IFDs)
        buf.extend_from_slice(&0u32.to_le_bytes());

        // Verify position
        assert_eq!(buf.len(), strip_data_offset as usize, "offset mismatch");

        // Strip data (f32 values, little-endian)
        for &val in values {
            buf.extend_from_slice(&val.to_le_bytes());
        }

        buf
    }

    fn build_geotiff_f32_with_keys(
        width: u32,
        height: u32,
        values: &[f32],
        geo_keys: &[u16],
    ) -> Vec<u8> {
        assert_eq!(values.len(), (width * height) as usize);

        let mut buf = Vec::new();

        // TIFF Header
        buf.extend_from_slice(b"II");
        buf.extend_from_slice(&42u16.to_le_bytes());
        let ifd_offset_pos = buf.len();
        buf.extend_from_slice(&0u32.to_le_bytes());

        let ifd_start = buf.len();
        buf[ifd_offset_pos..ifd_offset_pos + 4].copy_from_slice(&(ifd_start as u32).to_le_bytes());

        // We need space for: IFD entries + next_ifd + GeoKeyDirectory data + strip data
        // 11 entries: Width, Height, BitsPerSample, Compression, StripOffsets, RowsPerStrip, StripByteCounts, SampleFormat, GeoKeyDir
        let num_entries: u16 = 9;
        buf.extend_from_slice(&num_entries.to_le_bytes());

        let entries_end = buf.len() + num_entries as usize * 12 + 4; // +4 for next IFD
        let geo_key_offset = entries_end as u32;
        let geo_key_size = geo_keys.len() * 2; // u16 each

        let strip_data_offset = geo_key_offset + geo_key_size as u32;
        let strip_data_size = values.len() * 4;

        // Tag 256: ImageWidth
        buf.extend_from_slice(&256u16.to_le_bytes());
        buf.extend_from_slice(&4u16.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&width.to_le_bytes());

        // Tag 257: ImageLength
        buf.extend_from_slice(&257u16.to_le_bytes());
        buf.extend_from_slice(&4u16.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&height.to_le_bytes());

        // Tag 258: BitsPerSample
        buf.extend_from_slice(&258u16.to_le_bytes());
        buf.extend_from_slice(&3u16.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&32u16.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // padding

        // Tag 259: Compression
        buf.extend_from_slice(&259u16.to_le_bytes());
        buf.extend_from_slice(&3u16.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // padding

        // Tag 273: StripOffsets
        buf.extend_from_slice(&273u16.to_le_bytes());
        buf.extend_from_slice(&4u16.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&strip_data_offset.to_le_bytes());

        // Tag 278: RowsPerStrip
        buf.extend_from_slice(&278u16.to_le_bytes());
        buf.extend_from_slice(&4u16.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&height.to_le_bytes());

        // Tag 279: StripByteCounts
        buf.extend_from_slice(&279u16.to_le_bytes());
        buf.extend_from_slice(&4u16.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&(strip_data_size as u32).to_le_bytes());

        // Tag 339: SampleFormat (3 = IEEE floating point)
        buf.extend_from_slice(&339u16.to_le_bytes());
        buf.extend_from_slice(&3u16.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&3u16.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // padding

        // Tag 34735: GeoKeyDirectory
        buf.extend_from_slice(&34735u16.to_le_bytes());
        buf.extend_from_slice(&3u16.to_le_bytes()); // SHORT
        buf.extend_from_slice(&(geo_keys.len() as u32).to_le_bytes());
        buf.extend_from_slice(&geo_key_offset.to_le_bytes());

        // Next IFD
        buf.extend_from_slice(&0u32.to_le_bytes());

        // GeoKeyDirectory data
        for &key in geo_keys {
            buf.extend_from_slice(&key.to_le_bytes());
        }

        // Strip data
        for &val in values {
            buf.extend_from_slice(&val.to_le_bytes());
        }

        buf
    }

    // -------------------------------------------------------------------------
    // Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_minimal_tiff_f32() {
        let values = vec![10.0f32, 20.0, 30.0, 40.0];
        let tiff = build_minimal_tiff_f32(2, 2, &values);
        let info = parse_geotiff_core(&tiff).unwrap();

        assert_eq!(info.width(), 2);
        assert_eq!(info.height(), 2);
        assert_eq!(info.inner.bits_per_sample, 32);
        assert_eq!(info.inner.sample_format, 1); // Default
        assert_eq!(info.inner.compression, 1); // None

        assert_eq!(info.elevations.len(), 4);
        assert_eq!(info.elevations[0], 10.0);
        assert_eq!(info.elevations[1], 20.0);
        assert_eq!(info.elevations[2], 30.0);
        assert_eq!(info.elevations[3], 40.0);
    }

    #[test]
    fn test_parse_tiff_byte_order_big_endian() {
        // Big-endian TIFF header
        let mut buf = Vec::new();
        buf.extend_from_slice(b"MM"); // Big endian
        buf.extend_from_slice(&42u16.to_be_bytes()); // Magic
        buf.extend_from_slice(&8u32.to_be_bytes()); // IFD at offset 8

        // IFD: 1 entry
        buf.extend_from_slice(&1u16.to_be_bytes()); // num entries
                                                    // Tag 256: ImageWidth = 1
        buf.extend_from_slice(&256u16.to_be_bytes());
        buf.extend_from_slice(&3u16.to_be_bytes()); // SHORT
        buf.extend_from_slice(&1u32.to_be_bytes()); // count
        buf.extend_from_slice(&1u16.to_be_bytes()); // value
        buf.extend_from_slice(&0u16.to_be_bytes()); // padding
                                                    // Next IFD = 0
        buf.extend_from_slice(&0u32.to_be_bytes());

        // Should parse header but fail on missing ImageLength
        let result = parse_geotiff_impl(&buf);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("missing ImageWidth or ImageLength"));
    }

    #[test]
    fn test_parse_geotiff_with_geokeys() {
        let values = vec![100.0f32, 200.0, 150.0, 250.0];
        let geo_keys: Vec<u16> = vec![1, 1, 0, 2, 1024, 0, 1, 2, 2048, 0, 1, 4326];

        let tiff = build_geotiff_f32_with_keys(2, 2, &values, &geo_keys);
        let info = parse_geotiff_core(&tiff).unwrap();

        assert_eq!(info.width(), 2);
        assert_eq!(info.height(), 2);
        assert_eq!(info.inner.bits_per_sample, 32);
        assert_eq!(info.inner.sample_format, 3); // IEEE FP

        assert_eq!(info.elevations[0], 100.0);
        assert_eq!(info.elevations[3], 250.0);

        let model_type = info.get_geokey_short(geokey::GT_MODEL_TYPE_GEO_KEY);
        assert_eq!(model_type, Some(model_type::GEOGRAPHIC));

        let geo_type = info.get_geokey_short(geokey::GEOGRAPHIC_TYPE_GEO_KEY);
        assert_eq!(geo_type, Some(crs_code::WGS_84));
    }

    #[test]
    fn test_parse_geotiff_larger_grid() {
        let values: Vec<f32> = (0..12).map(|i| (i as f32) * 10.0).collect();
        let tiff = build_minimal_tiff_f32(4, 3, &values);
        let info = parse_geotiff_core(&tiff).unwrap();

        assert_eq!(info.width(), 4);
        assert_eq!(info.height(), 3);
        assert_eq!(info.elevations.len(), 12);

        for i in 0..12 {
            assert!((info.elevations[i] - (i as f32) * 10.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_parse_geotiff_negative_elevations() {
        let values = vec![-10.0f32, 0.0, 5.0, -3.5];
        let tiff = build_minimal_tiff_f32(2, 2, &values);
        let info = parse_geotiff_core(&tiff).unwrap();

        assert_eq!(info.elevations[0], -10.0);
        assert_eq!(info.elevations[3], -3.5);
    }

    #[test]
    fn test_parse_too_small() {
        let result = parse_geotiff_core(&[0u8; 4]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too small"));
    }

    #[test]
    fn test_parse_invalid_byte_order() {
        let mut buf = vec![0xFF, 0xFF]; // Invalid byte order
        buf.extend_from_slice(&42u16.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        let result = parse_geotiff_impl(&buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid byte order"));
    }

    #[test]
    fn test_parse_invalid_magic() {
        let mut buf = vec![0x49, 0x49]; // II
        buf.extend_from_slice(&99u16.to_le_bytes()); // Wrong magic
        buf.extend_from_slice(&0u32.to_le_bytes());
        let result = parse_geotiff_impl(&buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid magic"));
    }

    #[test]
    fn test_elevation_swath() {
        let values: Vec<f32> = (0..20).map(|i| i as f32).collect();
        let tiff = build_minimal_tiff_f32(4, 5, &values);
        let info = parse_geotiff_core(&tiff).unwrap();

        // All rows in one strip — verify full elevation is correct
        assert_eq!(info.elevations.len(), 20);
        assert_eq!(info.elevations[0], 0.0);
        assert_eq!(info.elevations[19], 19.0);
    }

    #[test]
    fn test_crs_json_format() {
        let values = vec![1.0f32, 2.0, 3.0, 4.0];
        let geo_keys: Vec<u16> = vec![
            1, 1, 0, 1, // header with 1 key
            2048, 0, 1, 4326, // GeographicTypeGeoKey = WGS 84
        ];
        let tiff = build_geotiff_f32_with_keys(2, 2, &values, &geo_keys);
        let info = parse_geotiff_core(&tiff).unwrap();

        let crs = info.crs();
        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&crs).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.get("geographicType").is_some());
        assert_eq!(parsed["geographicTypeCode"], 4326);
    }

    // -------------------------------------------------------------------------
    // Quantized-Mesh Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_encode_quantized_mesh_basic() {
        let heights = vec![
            100.0f32, 200.0, 150.0, 250.0, // 4x4 grid first row
            110.0, 210.0, 160.0, 260.0, 120.0, 220.0, 170.0, 270.0, 130.0, 230.0, 180.0, 280.0,
        ];
        let bounds = [116.0, 39.0, 117.0, 40.0];
        let center = [6378137.0, 0.0, 0.0];

        let data = encode_quantized_mesh_core(&heights, 4, 4, &bounds, &center).unwrap();

        // Header: 24 bytes (center xyz) + 2 + 2 + 1 + 1 + 4 = 34...
        // Actually: 8+8+8 = 24 (center), 2+2 (min/max height), 1+1 (oct/water), 4 (header_size) = 34
        // Vertex data: 4 (vertex_count) + 16*6 = 100
        // Index data: 4 (triangle_count=18) + 18*3*2 = 112 + edge indices
        assert!(data.len() > 100);

        // Verify vertex count (4x4=16)
        // vertex count is at offset after header_size
        let header_size = u32::from_le_bytes([data[30], data[31], data[32], data[33]]) as usize;
        assert_eq!(header_size, 34); // 24 + 2 + 2 + 1 + 1 + 4
        let vertex_count = u32::from_le_bytes([
            data[header_size],
            data[header_size + 1],
            data[header_size + 2],
            data[header_size + 3],
        ]);
        assert_eq!(vertex_count, 16);
    }

    #[test]
    fn test_encode_quantized_mesh_flat_terrain() {
        let heights = vec![100.0f32; 6]; // 3x2 grid, all same height
        let bounds = [0.0, 0.0, 1.0, 1.0];
        let center = [0.0, 0.0, 6378137.0];

        let data = encode_quantized_mesh_core(&heights, 3, 2, &bounds, &center).unwrap();
        assert!(!data.is_empty());

        // Should have 6 vertices, 4 triangles
        let header_size = u32::from_le_bytes([data[30], data[31], data[32], data[33]]) as usize;
        let vertex_count = u32::from_le_bytes([
            data[header_size],
            data[header_size + 1],
            data[header_size + 2],
            data[header_size + 3],
        ]);
        assert_eq!(vertex_count, 6);
    }

    #[test]
    fn test_encode_quantized_mesh_too_small() {
        let heights = vec![1.0f32, 2.0]; // 2x1 — too small
        let bounds = [0.0, 0.0, 1.0, 1.0];
        let center = [0.0, 0.0, 0.0];

        let result = encode_quantized_mesh_core(&heights, 2, 1, &bounds, &center);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least 2×2"));
    }

    #[test]
    fn test_encode_quantized_mesh_height_mismatch() {
        let heights = vec![1.0f32, 2.0, 3.0]; // 3 values for 2x2
        let bounds = [0.0, 0.0, 1.0, 1.0];
        let center = [0.0, 0.0, 0.0];

        let result = encode_quantized_mesh_core(&heights, 2, 2, &bounds, &center);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("length"));
    }

    // -------------------------------------------------------------------------
    // Terrain Tileset Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_encode_terrain_tileset_basic() {
        let heights: Vec<f32> = (0..16).map(|i| (i as f32) * 10.0).collect();
        let bounds = [116.0, 39.0, 117.0, 40.0];
        let center = [6378137.0, 0.0, 0.0];

        let result = encode_terrain_tileset_core(&heights, 4, 4, &bounds, &center, 2).unwrap();
        assert_eq!(result.tile_count(), 3); // zoom 0, 1, 2

        // Check tileset JSON is valid
        let json: serde_json::Value = serde_json::from_str(&result.tileset_json).unwrap();
        assert_eq!(json["asset"]["version"], "1.1");
        assert!(json["root"]["children"].is_array());
        assert_eq!(json["root"]["children"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_encode_terrain_tileset_downsampling() {
        let heights: Vec<f32> = (0..64).map(|i| i as f32).collect(); // 8x8
        let bounds = [0.0, 0.0, 1.0, 1.0];
        let center = [0.0, 0.0, 6378137.0];

        let result = encode_terrain_tileset_core(&heights, 8, 8, &bounds, &center, 3).unwrap();
        assert_eq!(result.tiles.len(), 4); // zoom 0..3

        // Level 0: 8x8, Level 1: 4x4, Level 2: 2x2, Level 3: 2x2 (min)
        assert!(result.tiles[0].len() > result.tiles[1].len());
    }

    #[test]
    fn test_encode_terrain_tileset_tile_uris() {
        let heights: Vec<f32> = (0..4).map(|i| i as f32).collect();
        let bounds = [0.0, 0.0, 1.0, 1.0];
        let center = [0.0, 0.0, 0.0];

        let result = encode_terrain_tileset_core(&heights, 2, 2, &bounds, &center, 2).unwrap();
        assert_eq!(result.tile_uri(0), "terrain_0.cmpt");
        assert_eq!(result.tile_uri(1), "terrain_1.cmpt");
        assert_eq!(result.tile_uri(2), "terrain_2.cmpt");
    }

    #[test]
    fn test_supports_geotiff() {
        assert!(supports_geotiff());
    }

    #[test]
    fn test_geotiff_status() {
        let status = geotiff_status();
        assert!(status.contains("AVAILABLE"));
        assert!(status.contains("DEFLATE"));
    }

    #[test]
    fn test_field_type_sizes() {
        assert_eq!(field_type_size(field_type::BYTE), 1);
        assert_eq!(field_type_size(field_type::SHORT), 2);
        assert_eq!(field_type_size(field_type::LONG), 4);
        assert_eq!(field_type_size(field_type::FLOAT), 4);
        assert_eq!(field_type_size(field_type::DOUBLE), 8);
        assert_eq!(field_type_size(field_type::RATIONAL), 8);
    }

    #[test]
    fn test_geokey_parsing() {
        let dir: Vec<u16> = vec![
            1, 1, 0, 2, // header: version=1, rev=1, minor=0, count=2
            1024, 0, 1, 2, // GTModelTypeGeoKey = Geographic
            2048, 0, 1, 4326, // GeographicTypeGeoKey = WGS84
        ];
        let keys = parse_geo_keys(&dir, "");
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].0, 1024);
        if let StringOrShort::Short(v) = &keys[0].3 {
            assert_eq!(*v, 2);
        } else {
            panic!("Expected short value");
        }
        assert_eq!(keys[1].0, 2048);
        if let StringOrShort::Short(v) = &keys[1].3 {
            assert_eq!(*v, 4326);
        } else {
            panic!("Expected short value");
        }
    }

    #[test]
    fn test_decompress_none() {
        let data = vec![1u8, 2, 3, 4];
        let result = decompress_data(&data, compression::NONE).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_decompress_deflate() {
        use std::io::Write;
        let original = vec![10u8, 20, 30, 40, 50];
        let mut compressed = Vec::new();
        let mut encoder =
            flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::fast());
        encoder.write_all(&original).unwrap();
        encoder.finish().unwrap();

        let result = decompress_data(&compressed, compression::DEFLATE).unwrap();
        assert_eq!(result, original);
    }

    #[test]
    fn test_decompress_lzw_unsupported() {
        let result = decompress_data(&[1, 2, 3], compression::LZW);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("LZW"));
    }
}
