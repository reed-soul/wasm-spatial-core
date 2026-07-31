//! Structured error codes for WASM-visible error handling.
//!
//! Replaces raw `JsValue::from_str(...)` with typed `SpatialError` enums,
//! enabling programmatic error handling from JavaScript.
//!
//! ```js
//! try {
//!   core.parseGeoJsonCoords(input);
//! } catch (e) {
//!   if (e.code === 'PARSE_ERROR') { ... }
//! }
//! ```

use wasm_bindgen::prelude::*;

// ===========================================================================
// Error enumeration — NOT #[wasm_bindgen] enum to avoid auto-generated
// From<JsValue> conflicts. We manually convert to JsValue.
// ===========================================================================

/// Typed spatial error with machine-readable code and human-readable message.
#[derive(Debug, Clone)]
pub enum SpatialError {
    /// Input format is invalid (wrong type, malformed, etc.)
    InvalidInput,
    /// Input exceeds the 100 MB safety limit
    InputTooLarge,
    /// JSON / GeoJSON parsing failed
    ParseError,
    /// Geometry computation failed (e.g. earcut triangulation)
    GeometryError,
    /// Spatial index is empty or index out of bounds
    IndexError,
    /// MVT / vector tile generation failed
    TileError,
    /// Point cloud (LAS / PCD) parsing failed
    PointCloudError,
    /// GeoTIFF terrain processing failed
    TerrainError,
    /// Long-running operation was cancelled via abort token
    Cancelled,
}

impl SpatialError {
    /// Machine-readable error code string.
    pub fn code(&self) -> &'static str {
        match self {
            SpatialError::InvalidInput => "INVALID_INPUT",
            SpatialError::InputTooLarge => "INPUT_TOO_LARGE",
            SpatialError::ParseError => "PARSE_ERROR",
            SpatialError::GeometryError => "GEOMETRY_ERROR",
            SpatialError::IndexError => "INDEX_ERROR",
            SpatialError::TileError => "TILE_ERROR",
            SpatialError::PointCloudError => "POINT_CLOUD_ERROR",
            SpatialError::TerrainError => "TERRAIN_ERROR",
            SpatialError::Cancelled => "CANCELLED",
        }
    }

    /// Human-readable error description.
    pub fn description(&self) -> &'static str {
        match self {
            SpatialError::InvalidInput => "Invalid input: format or type is incorrect",
            SpatialError::InputTooLarge => "Input exceeds the maximum allowed size (100 MB)",
            SpatialError::ParseError => "Failed to parse JSON or GeoJSON data",
            SpatialError::GeometryError => "Geometry computation failed",
            SpatialError::IndexError => "Spatial index is empty or out of bounds",
            SpatialError::TileError => "Vector tile generation failed",
            SpatialError::PointCloudError => "Point cloud (LAS/PCD) parsing failed",
            SpatialError::TerrainError => "GeoTIFF terrain processing failed",
            SpatialError::Cancelled => "Operation was cancelled",
        }
    }

    /// Create an error with a custom detail message appended.
    pub fn with_detail(&self, detail: impl std::fmt::Display) -> SpatialErrorDetail {
        SpatialErrorDetail {
            kind: self.clone(),
            detail: detail.to_string(),
        }
    }

    /// Convenience: InvalidInput with detail
    pub fn invalid_input(detail: impl std::fmt::Display) -> SpatialErrorDetail {
        SpatialError::InvalidInput.with_detail(detail)
    }
    /// Convenience: InputTooLarge with detail
    pub fn input_too_large(detail: impl std::fmt::Display) -> SpatialErrorDetail {
        SpatialError::InputTooLarge.with_detail(detail)
    }
    /// Convenience: ParseError with detail
    pub fn parse_error(detail: impl std::fmt::Display) -> SpatialErrorDetail {
        SpatialError::ParseError.with_detail(detail)
    }
    /// Convenience: GeometryError with detail
    pub fn geometry_error(detail: impl std::fmt::Display) -> SpatialErrorDetail {
        SpatialError::GeometryError.with_detail(detail)
    }
    /// Convenience: TileError with detail
    pub fn tile_error(detail: impl std::fmt::Display) -> SpatialErrorDetail {
        SpatialError::TileError.with_detail(detail)
    }
    /// Convenience: PointCloudError with detail
    pub fn point_cloud_error(detail: impl std::fmt::Display) -> SpatialErrorDetail {
        SpatialError::PointCloudError.with_detail(detail)
    }
    /// Convenience: IndexError with detail
    pub fn index_error(detail: impl std::fmt::Display) -> SpatialErrorDetail {
        SpatialError::IndexError.with_detail(detail)
    }
    /// Convenience: TerrainError with detail
    pub fn terrain_error(detail: impl std::fmt::Display) -> SpatialErrorDetail {
        SpatialError::TerrainError.with_detail(detail)
    }
    /// Convenience: Cancelled with detail
    pub fn cancelled(detail: impl std::fmt::Display) -> SpatialErrorDetail {
        SpatialError::Cancelled.with_detail(detail)
    }
}

impl std::fmt::Display for SpatialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.description())
    }
}

impl std::error::Error for SpatialError {}

// ===========================================================================
// SpatialErrorDetail — error with additional context
// ===========================================================================

/// A `SpatialError` with additional context detail.
#[derive(Debug, Clone)]
pub struct SpatialErrorDetail {
    kind: SpatialError,
    detail: String,
}

impl SpatialErrorDetail {
    /// Machine-readable error code (e.g. `"PARSE_ERROR"`).
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// Human-readable error description including detail.
    pub fn message(&self) -> String {
        format!(
            "[{}] {}: {}",
            self.kind.code(),
            self.kind.description(),
            self.detail
        )
    }
}

impl std::fmt::Display for SpatialErrorDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for SpatialErrorDetail {}

// ===========================================================================
// Conversions to JsValue — structured error objects for JS consumption
// ===========================================================================

fn error_to_js(code: &str, message: &str) -> JsValue {
    // Build a real `js_sys::Error` (not a plain `Object`) so JS code using
    // `e instanceof Error`, `e.stack`, or `console.error` formatting works.
    // Attach `code`/`name` as custom properties.
    let err = js_sys::Error::new(message);
    // The constructor sets `message` already; reflect-set the rest. These
    // cannot realistically fail on a fresh Error, so use `debug_assert` to
    // surface bugs in debug builds without aborting release on a JS quirk.
    let code_val = JsValue::from(code);
    let name_val = JsValue::from("SpatialError");
    debug_assert!(
        js_sys::Reflect::set(&err, &"code".into(), &code_val).is_ok(),
        "Reflect::set(code) failed"
    );
    debug_assert!(
        js_sys::Reflect::set(&err, &"name".into(), &name_val).is_ok(),
        "Reflect::set(name) failed"
    );
    err.into()
}

impl From<SpatialError> for JsValue {
    fn from(err: SpatialError) -> JsValue {
        error_to_js(err.code(), err.description())
    }
}

impl From<SpatialErrorDetail> for JsValue {
    fn from(err: SpatialErrorDetail) -> JsValue {
        error_to_js(err.code(), &err.message())
    }
}

// ===========================================================================
// Helpers for `Result<_, JsValue>` in wasm-bindgen exports
// ===========================================================================

#[inline]
pub(crate) fn parse_js(detail: impl std::fmt::Display) -> JsValue {
    SpatialError::parse_error(detail).into()
}

#[inline]
pub(crate) fn invalid_input_js(detail: impl std::fmt::Display) -> JsValue {
    SpatialError::invalid_input(detail).into()
}

#[inline]
pub(crate) fn input_too_large_js(detail: impl std::fmt::Display) -> JsValue {
    SpatialError::InputTooLarge.with_detail(detail).into()
}

#[inline]
pub(crate) fn tile_js(detail: impl std::fmt::Display) -> JsValue {
    SpatialError::tile_error(detail).into()
}

#[inline]
pub(crate) fn geometry_js(detail: impl std::fmt::Display) -> JsValue {
    SpatialError::geometry_error(detail).into()
}

#[inline]
#[allow(dead_code)]
pub(crate) fn terrain_js(detail: impl std::fmt::Display) -> JsValue {
    SpatialError::terrain_error(detail).into()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(SpatialError::InvalidInput.code(), "INVALID_INPUT");
        assert_eq!(SpatialError::InputTooLarge.code(), "INPUT_TOO_LARGE");
        assert_eq!(SpatialError::ParseError.code(), "PARSE_ERROR");
        assert_eq!(SpatialError::GeometryError.code(), "GEOMETRY_ERROR");
        assert_eq!(SpatialError::IndexError.code(), "INDEX_ERROR");
        assert_eq!(SpatialError::TileError.code(), "TILE_ERROR");
        assert_eq!(SpatialError::PointCloudError.code(), "POINT_CLOUD_ERROR");
        assert_eq!(SpatialError::TerrainError.code(), "TERRAIN_ERROR");
        assert_eq!(SpatialError::Cancelled.code(), "CANCELLED");
    }

    #[test]
    fn test_error_descriptions_non_empty() {
        for variant in &[
            SpatialError::InvalidInput,
            SpatialError::InputTooLarge,
            SpatialError::ParseError,
            SpatialError::GeometryError,
            SpatialError::IndexError,
            SpatialError::TileError,
            SpatialError::PointCloudError,
            SpatialError::TerrainError,
            SpatialError::Cancelled,
        ] {
            assert!(!variant.description().is_empty());
            assert!(variant.description().len() > 10);
        }
    }

    #[test]
    fn test_error_detail_message() {
        let err = SpatialError::parse_error("unexpected token at position 42");
        assert_eq!(err.code(), "PARSE_ERROR");
        assert!(err.message().contains("unexpected token at position 42"));
        assert!(err.message().contains("Failed to parse"));
    }

    #[test]
    fn test_convenience_constructors() {
        let err = SpatialError::invalid_input("empty string");
        assert_eq!(err.code(), "INVALID_INPUT");
        assert!(err.message().contains("empty string"));

        let err = SpatialError::geometry_error("earcut returned empty");
        assert_eq!(err.code(), "GEOMETRY_ERROR");

        let err = SpatialError::point_cloud_error("not a LAS file");
        assert_eq!(err.code(), "POINT_CLOUD_ERROR");

        let err = SpatialError::tile_error("layer missing");
        assert_eq!(err.code(), "TILE_ERROR");

        let err = SpatialError::index_error("no points in index");
        assert_eq!(err.code(), "INDEX_ERROR");
    }

    #[test]
    fn test_display_impl() {
        let err = SpatialError::ParseError;
        let display = format!("{err}");
        assert!(display.contains("PARSE_ERROR"));
        assert!(display.contains("Failed to parse"));
    }

    #[test]
    fn test_detail_display_impl() {
        let err = SpatialError::tile_error("zoom level too high");
        let display = format!("{err}");
        assert!(display.contains("TILE_ERROR"));
        assert!(display.contains("zoom level too high"));
    }
}
