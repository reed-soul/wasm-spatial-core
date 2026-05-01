/**
 * wasm-spatial-core — TypeScript convenience wrapper
 *
 * Re-exports the auto-generated wasm-bindgen bindings with a
 * higher-level initialisation helper and typed interfaces.
 *
 * @packageDocumentation
 * @author  Qingxi
 * @license MIT
 * @copyright 2026 智启未来 (Zhiqi Weilai)
 */

// Re-export everything from the auto-generated wasm-bindgen module
export {
  default as initWasm,
  version,
  batchWgs84ToGcj02,
  batchGcj02ToWgs84,
  batchWgs84ToMercator,
  parseGeoJsonCoords,
  countGeoJsonFeatures,
} from "./wasm_spatial_core.js";

// ---------------------------------------------------------------------------
// Convenience types
// ---------------------------------------------------------------------------

/** Supported coordinate reference systems. */
export type CRS =
  | "WGS84"      // EPSG:4326
  | "GCJ02"      // China encrypted
  | "BD09"       // Baidu (planned)
  | "EPSG:3857"; // Web Mercator

/** Options for batch coordinate conversion. */
export interface ConvertOptions {
  /** Source CRS — defaults to `"WGS84"`. */
  from?: CRS;
  /** Target CRS — defaults to `"GCJ02"`. */
  to?: CRS;
}

/**
 * High-level helper: initialise the WASM module and return the public API.
 *
 * ```ts
 * import { loadSpatialCore } from "@anthropic-wasm/spatial-core";
 *
 * const core = await loadSpatialCore();
 * console.log(core.version());
 * ```
 */
export async function loadSpatialCore() {
  const { default: init, ...api } = await import("./wasm_spatial_core.js");
  await init();
  return api;
}
