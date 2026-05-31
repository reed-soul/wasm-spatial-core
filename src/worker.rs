//! # WebWorker Parallel Processing
//!
//! Inline WebWorker creation for true parallel point cloud processing.
//! The main thread creates a Blob URL Worker that loads the WASM module
//! independently, builds the octree, and generates tiles — all without
//! blocking the main thread.
//!
//! ## Fallback
//!
//! When Web Workers are unavailable (e.g. no COOP/COEP headers for
//! `SharedArrayBuffer`), the module provides `processChunked` for
//! cooperative main-thread processing.

use wasm_bindgen::prelude::*;

// ===========================================================================
// Worker Options
// ===========================================================================

/// Configuration for point cloud processing in a Worker.
#[wasm_bindgen(js_name = "WorkerOptions")]
#[derive(Debug, Clone)]
pub struct WorkerOptions {
    max_points_per_node: u32,
    max_depth: u32,
}

#[wasm_bindgen(js_class = "WorkerOptions")]
impl WorkerOptions {
    /// Create new WorkerOptions with defaults.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            max_points_per_node: 50_000,
            max_depth: 21,
        }
    }

    /// Maximum points per octree leaf node (default: 50,000).
    #[wasm_bindgen(getter, js_name = "maxPointsPerNode")]
    pub fn max_points_per_node(&self) -> u32 {
        self.max_points_per_node
    }

    /// Set maximum points per octree leaf node.
    #[wasm_bindgen(setter, js_name = "maxPointsPerNode")]
    pub fn set_max_points_per_node(&mut self, value: u32) {
        self.max_points_per_node = value;
    }

    /// Maximum tree depth (default: 21).
    #[wasm_bindgen(getter, js_name = "maxDepth")]
    pub fn max_depth(&self) -> u32 {
        self.max_depth
    }

    /// Set maximum tree depth.
    #[wasm_bindgen(setter, js_name = "maxDepth")]
    pub fn set_max_depth(&mut self, value: u32) {
        self.max_depth = value;
    }
}

impl Default for WorkerOptions {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Worker Support Detection
// ===========================================================================

/// Check if Web Workers are available in the current environment.
///
/// Returns `true` if `Worker` is defined in the global scope.
#[wasm_bindgen(js_name = "supportsWorker")]
pub fn supports_worker() -> bool {
    let js = js_sys::eval(
        r#"
        (function() {
            try {
                return typeof Worker !== 'undefined';
            } catch (e) {
                return false;
            }
        })()
        "#,
    );
    match js {
        Ok(v) => v.is_truthy(),
        Err(_) => false,
    }
}

// ===========================================================================
// Inline Worker Script Generator
// ===========================================================================

/// Generate the inline Worker JavaScript code.
///
/// This script is embedded as a string and turned into a Blob URL.
/// The Worker loads the WASM module, then waits for commands:
/// - `init` — Load and initialize the WASM module
/// - `process` — Build octree + generate tileset
/// - `cancel` — Abort current processing
pub fn generate_worker_script() -> String {
    r#"
'use strict';
let wasm = null;
let cancelled = false;

self.onmessage = async function(e) {
    const { type, payload } = e.data;
    
    if (type === 'init') {
        try {
            const { default: init, buildOctree, generateTileset, parseGeotiff, applyTerrainColorRamp, hillshade, terrainToGlb } = await import(payload.wasmUrl);
            await init();
            wasm = { init, buildOctree, generateTileset, parseGeotiff, applyTerrainColorRamp, hillshade, terrainToGlb };
            self.postMessage({ type: 'ready' });
        } catch (err) {
            self.postMessage({ type: 'error', payload: { message: err.message, stage: 'init' } });
        }
    }
    
    if (type === 'process') {
        cancelled = false;
        try {
            const { positions, colors, options } = payload;
            
            // Report progress: 0% — starting octree build
            self.postMessage({ type: 'progress', payload: { stage: 'octree', progress: 0.0 } });
            
            // Build octree
            const octree = wasm.buildOctree(
                positions,
                options.maxPointsPerNode,
                options.maxDepth
            );
            
            if (cancelled) {
                self.postMessage({ type: 'cancelled' });
                return;
            }
            
            // Report progress: 50% — starting tileset generation
            self.postMessage({ type: 'progress', payload: { stage: 'tileset', progress: 0.5 } });
            
            // Generate tileset
            const tiles = wasm.generateTileset(
                positions,
                options.maxPointsPerNode,
                options.maxDepth,
                colors
            );
            
            if (cancelled) {
                self.postMessage({ type: 'cancelled' });
                return;
            }
            
            // Collect all tile ArrayBuffers for transfer
            const tileCount = tiles.tileCount;
            const tileBuffers = [];
            const tileSizes = [];
            for (let i = 0; i < tileCount; i++) {
                const tileData = tiles.tile(i);
                tileBuffers.push(tileData.buffer);
                tileSizes.push(tileData.byteLength);
            }
            
            // Report completion
            self.postMessage(
                {
                    type: 'complete',
                    payload: {
                        tilesetJson: tiles.tilesetJson(),
                        tileCount: tileCount,
                        totalBytes: tiles.totalBytes,
                        tileSizes: tileSizes
                    }
                },
                tileBuffers // Transferable
            );
        } catch (err) {
            self.postMessage({ type: 'error', payload: { message: err.message, stage: 'process' } });
        }
    }
    
    if (type === 'cancel') {
        cancelled = true;
    }
    
    if (type === 'terrain') {
        cancelled = false;
        try {
            const { geotiffBytes, options } = payload;
            
            self.postMessage({ type: 'progress', payload: { stage: 'parse', progress: 0.0 } });
            
            if (cancelled) { self.postMessage({ type: 'cancelled' }); return; }
            
            const info = wasm.parseGeotiff(geotiffBytes);
            
            self.postMessage({ type: 'progress', payload: { stage: 'style', progress: 0.3 } });
            
            if (cancelled) { self.postMessage({ type: 'cancelled' }); return; }
            
            // Apply color ramp if requested
            let colorRgba = null;
            if (options && options.colorRamp !== undefined) {
                const bounds = info.bounds();
                colorRgba = wasm.applyTerrainColorRamp(
                    info.elevation(),
                    bounds[0], bounds[1],
                    options.colorRamp
                );
            }
            
            // Compute hillshade if requested
            let hillshadeData = null;
            if (options && options.hillshade) {
                hillshadeData = wasm.hillshade(
                    info.elevation(),
                    info.width(),
                    info.height(),
                    options.azimuth || 315.0,
                    options.altitude || 45.0
                );
            }
            
            self.postMessage({ type: 'progress', payload: { stage: 'mesh', progress: 0.6 } });
            
            if (cancelled) { self.postMessage({ type: 'cancelled' }); return; }
            
            // Generate terrain mesh as GLB
            const bounds = info.bounds();
            const elevation = info.elevation();
            const glbData = wasm.terrainToGlb(elevation, info.width(), info.height(), bounds);
            
            // Collect all data for transfer
            const buffers = [glbData.buffer];
            if (colorRgba) buffers.push(colorRgba.buffer);
            if (hillshadeData) buffers.push(hillshadeData.buffer);
            
            self.postMessage(
                {
                    type: 'complete',
                    payload: {
                        glb: glbData,
                        colorRgba: colorRgba,
                        hillshade: hillshadeData,
                        width: info.width(),
                        height: info.height(),
                        bounds: Array.from(bounds),
                        elevationCount: elevation.length
                    }
                },
                buffers
            );
        } catch (err) {
            self.postMessage({ type: 'error', payload: { message: err.message, stage: 'terrain' } });
        }
    }
};
"#
    .to_string()
}

// ===========================================================================
// Worker Handle
// ===========================================================================

/// A handle to a point cloud processing Web Worker.
///
/// Create via `createPointCloudWorker(wasmUrl)`. The Worker loads the WASM
/// module in a separate thread and executes the full Octree → Tileset pipeline.
///
/// # Example (JS)
/// ```js
/// const worker = createPointCloudWorker('https://example.com/spatial_core_bg.wasm');
/// worker.onProgress((stage, progress) => {
///     console.log(`${stage}: ${(progress * 100).toFixed(1)}%`);
/// });
/// worker.onComplete((result) => {
///     console.log(`Generated ${result.tileCount} tiles`);
/// });
/// worker.onError((err) => {
///     console.error('Worker error:', err);
/// });
/// worker.process(positions, colors, options);
/// ```
#[wasm_bindgen(js_name = "WorkerHandle")]
pub struct WorkerHandle {
    worker: web_sys::Worker,
    wasm_url: String,
    on_progress: Option<js_sys::Function>,
    on_complete: Option<js_sys::Function>,
    on_error: Option<js_sys::Function>,
    on_cancelled: Option<js_sys::Function>,
}

#[wasm_bindgen(js_class = "WorkerHandle")]
impl WorkerHandle {
    /// Create a new inline Worker for point cloud processing.
    ///
    /// # Arguments
    /// * `wasmUrl` — URL to the WASM module file (`.wasm`).
    ///
    /// The Worker is created as a Blob URL from an inline script. It loads
    /// the WASM module, initializes it, and waits for `process` commands.
    #[wasm_bindgen(constructor, js_name = "createPointCloudWorker")]
    pub fn new(wasm_url: &str) -> Result<WorkerHandle, JsValue> {
        let worker_script = generate_worker_script();

        // Create Blob from Uint8Array
        let script_bytes = js_sys::Uint8Array::new_with_length(worker_script.len() as u32);
        script_bytes.copy_from(worker_script.as_bytes());

        let blob_parts = js_sys::Array::new_with_length(1);
        blob_parts.set(0, script_bytes.into());

        let blob = web_sys::Blob::new_with_u8_array_sequence(&blob_parts)
            .map_err(|e| JsValue::from_str(&format!("Failed to create Blob: {e:?}")))?;

        let url = web_sys::Url::create_object_url_with_blob(&blob)
            .map_err(|e| JsValue::from_str(&format!("Failed to create Blob URL: {e:?}")))?;

        // Create Worker from URL string
        let worker = web_sys::Worker::new(&url)
            .map_err(|e| JsValue::from_str(&format!("Failed to create Worker: {e:?}")))?;

        // Release the Blob URL
        web_sys::Url::revoke_object_url(&url)
            .map_err(|e| JsValue::from_str(&format!("Failed to revoke Blob URL: {e:?}")))?;

        Ok(WorkerHandle {
            worker,
            wasm_url: wasm_url.to_string(),
            on_progress: None,
            on_complete: None,
            on_error: None,
            on_cancelled: None,
        })
    }

    /// Initialize the Worker (load and initialize WASM).
    ///
    /// Must be called before `process`. The Worker will post a `ready`
    /// message when initialization is complete.
    pub fn init(&self) -> Result<(), JsValue> {
        let msg = js_sys::Object::new();
        js_sys::Reflect::set(&msg, &"type".into(), &"init".into())?;

        let payload = js_sys::Object::new();
        js_sys::Reflect::set(
            &payload,
            &"wasmUrl".into(),
            &JsValue::from_str(&self.wasm_url),
        )?;
        js_sys::Reflect::set(&msg, &"payload".into(), &payload)?;

        self.worker.post_message(&msg)?;
        Ok(())
    }

    /// Submit a point cloud for processing in the Worker.
    ///
    /// Positions and colors are transferred (zero-copy) to the Worker.
    ///
    /// # Arguments
    /// * `positions` — `Float32Array` of `[x, y, z, ...]`.
    /// * `colors` — Optional `Uint8Array` of `[r, g, b, ...]`.
    /// * `options` — `WorkerOptions` for octree configuration.
    pub fn process(
        &mut self,
        positions: &js_sys::Float32Array,
        colors: Option<Vec<u8>>,
        options: &WorkerOptions,
    ) -> Result<(), JsValue> {
        // Set up message handler to dispatch to callbacks
        let on_progress = self.on_progress.clone();
        let on_complete = self.on_complete.clone();
        let on_error = self.on_error.clone();
        let on_cancelled = self.on_cancelled.clone();

        let closure = Closure::<dyn Fn(JsValue)>::new(move |event: JsValue| {
            let event = web_sys::MessageEvent::from(event);
            if let Ok(data) = event.data().dyn_into::<js_sys::Object>() {
                if let Ok(msg_type) = js_sys::Reflect::get(&data, &"type".into()) {
                    let type_str = msg_type.as_string().unwrap_or_default();

                    match type_str.as_str() {
                        "progress" => {
                            if let Some(ref cb) = on_progress {
                                if let Ok(payload) = js_sys::Reflect::get(&data, &"payload".into())
                                {
                                    let stage = js_sys::Reflect::get(&payload, &"stage".into())
                                        .unwrap_or(JsValue::UNDEFINED);
                                    let progress =
                                        js_sys::Reflect::get(&payload, &"progress".into())
                                            .unwrap_or(JsValue::UNDEFINED);
                                    let _ = cb.call2(&JsValue::NULL, &stage, &progress);
                                }
                            }
                        }
                        "complete" => {
                            if let Some(ref cb) = on_complete {
                                if let Ok(payload) = js_sys::Reflect::get(&data, &"payload".into())
                                {
                                    let _ = cb.call1(&JsValue::NULL, &payload);
                                }
                            }
                        }
                        "error" => {
                            if let Some(ref cb) = on_error {
                                if let Ok(payload) = js_sys::Reflect::get(&data, &"payload".into())
                                {
                                    let _ = cb.call1(&JsValue::NULL, &payload);
                                }
                            }
                        }
                        "cancelled" => {
                            if let Some(ref cb) = on_cancelled {
                                let _ = cb.call0(&JsValue::NULL);
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

        self.worker
            .set_onmessage(Some(closure.as_ref().unchecked_ref()));
        closure.forget();

        // Build process message
        let msg = js_sys::Object::new();
        js_sys::Reflect::set(&msg, &"type".into(), &"process".into())?;

        let payload = js_sys::Object::new();

        // Set positions
        js_sys::Reflect::set(&payload, &"positions".into(), &positions.into())?;

        // Set colors if present
        if let Some(ref colors_vec) = colors {
            let colors_arr = js_sys::Uint8Array::new_with_length(colors_vec.len() as u32);
            colors_arr.copy_from(colors_vec);
            js_sys::Reflect::set(&payload, &"colors".into(), &colors_arr.into())?;
        }

        // Options
        let opts_obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &opts_obj,
            &"maxPointsPerNode".into(),
            &JsValue::from(options.max_points_per_node),
        )?;
        js_sys::Reflect::set(
            &opts_obj,
            &"maxDepth".into(),
            &JsValue::from(options.max_depth),
        )?;
        js_sys::Reflect::set(&payload, &"options".into(), &opts_obj)?;

        js_sys::Reflect::set(&msg, &"payload".into(), &payload)?;

        self.worker.post_message(&msg)?;
        Ok(())
    }

    /// Register a progress callback.
    ///
    /// Callback receives `(stage: string, progress: number)` where `stage`
    /// is `"octree"` or `"tileset"` and `progress` is 0.0 to 1.0.
    #[wasm_bindgen(js_name = "onProgress")]
    pub fn on_progress(&mut self, callback: &js_sys::Function) {
        self.on_progress = Some(callback.clone());
    }

    /// Register a completion callback.
    ///
    /// Callback receives the result object with `tilesetJson`, `tileCount`,
    /// `totalBytes`, and `tileSizes`.
    #[wasm_bindgen(js_name = "onComplete")]
    pub fn on_complete(&mut self, callback: &js_sys::Function) {
        self.on_complete = Some(callback.clone());
    }

    /// Register an error callback.
    ///
    /// Callback receives an error object with `message` and `stage`.
    #[wasm_bindgen(js_name = "onError")]
    pub fn on_error(&mut self, callback: &js_sys::Function) {
        self.on_error = Some(callback.clone());
    }

    /// Register a cancellation callback.
    ///
    /// Called when the Worker is cancelled mid-processing.
    #[wasm_bindgen(js_name = "onCancelled")]
    pub fn on_cancelled(&mut self, callback: &js_sys::Function) {
        self.on_cancelled = Some(callback.clone());
    }

    /// Submit a GeoTIFF for terrain processing in the Worker.
    ///
    /// The Worker will parse the GeoTIFF, optionally apply color ramp
    /// and hillshade, and generate a GLB terrain mesh.
    ///
    /// # Arguments
    /// * `geotiff_bytes` — `Uint8Array` of raw GeoTIFF data.
    /// * `color_ramp` — Optional color ramp (0=Terrain, 1=Heat, 2=Ocean, 3=Gray), or `None`.
    /// * `azimuth` — Hillshade light azimuth (degrees, 0=N, 90=E). Default 315.
    /// * `altitude` — Hillshade light altitude (degrees). Default 45.
    #[wasm_bindgen(js_name = "processTerrain")]
    pub fn process_terrain(
        &mut self,
        geotiff_bytes: &js_sys::Uint8Array,
        color_ramp: Option<u32>,
        azimuth: Option<f64>,
        altitude: Option<f64>,
    ) -> Result<(), JsValue> {
        // Set up message handler to dispatch to callbacks
        let on_progress = self.on_progress.clone();
        let on_complete = self.on_complete.clone();
        let on_error = self.on_error.clone();
        let on_cancelled = self.on_cancelled.clone();

        let closure = Closure::<dyn Fn(JsValue)>::new(move |event: JsValue| {
            let event = web_sys::MessageEvent::from(event);
            if let Ok(data) = event.data().dyn_into::<js_sys::Object>() {
                if let Ok(msg_type) = js_sys::Reflect::get(&data, &"type".into()) {
                    let type_str = msg_type.as_string().unwrap_or_default();

                    match type_str.as_str() {
                        "progress" => {
                            if let Some(ref cb) = on_progress {
                                if let Ok(payload) = js_sys::Reflect::get(&data, &"payload".into())
                                {
                                    let stage = js_sys::Reflect::get(&payload, &"stage".into())
                                        .unwrap_or(JsValue::UNDEFINED);
                                    let progress =
                                        js_sys::Reflect::get(&payload, &"progress".into())
                                            .unwrap_or(JsValue::UNDEFINED);
                                    let _ = cb.call2(&JsValue::NULL, &stage, &progress);
                                }
                            }
                        }
                        "complete" => {
                            if let Some(ref cb) = on_complete {
                                if let Ok(payload) = js_sys::Reflect::get(&data, &"payload".into())
                                {
                                    let _ = cb.call1(&JsValue::NULL, &payload);
                                }
                            }
                        }
                        "error" => {
                            if let Some(ref cb) = on_error {
                                if let Ok(payload) = js_sys::Reflect::get(&data, &"payload".into())
                                {
                                    let _ = cb.call1(&JsValue::NULL, &payload);
                                }
                            }
                        }
                        "cancelled" => {
                            if let Some(ref cb) = on_cancelled {
                                let _ = cb.call0(&JsValue::NULL);
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

        self.worker
            .set_onmessage(Some(closure.as_ref().unchecked_ref()));
        closure.forget();

        // Build terrain message
        let msg = js_sys::Object::new();
        js_sys::Reflect::set(&msg, &"type".into(), &"terrain".into())?;

        let payload = js_sys::Object::new();
        js_sys::Reflect::set(&payload, &"geotiffBytes".into(), &geotiff_bytes.into())?;

        let opts_obj = js_sys::Object::new();
        if let Some(ramp) = color_ramp {
            js_sys::Reflect::set(&opts_obj, &"colorRamp".into(), &JsValue::from(ramp))?;
            js_sys::Reflect::set(&opts_obj, &"hillshade".into(), &JsValue::from(true))?;
        }
        if let Some(az) = azimuth {
            js_sys::Reflect::set(&opts_obj, &"azimuth".into(), &JsValue::from(az))?;
        }
        if let Some(alt) = altitude {
            js_sys::Reflect::set(&opts_obj, &"altitude".into(), &JsValue::from(alt))?;
        }
        js_sys::Reflect::set(&payload, &"options".into(), &opts_obj)?;

        js_sys::Reflect::set(&msg, &"payload".into(), &payload)?;

        self.worker.post_message(&msg)?;
        Ok(())
    }

    /// Cancel the current processing job.
    ///
    /// The Worker will stop as soon as possible (between octree build
    /// and tileset generation phases).
    #[wasm_bindgen(js_name = "cancel")]
    pub fn cancel(&self) -> Result<(), JsValue> {
        let msg = js_sys::Object::new();
        js_sys::Reflect::set(&msg, &"type".into(), &"cancel".into())?;
        self.worker.post_message(&msg)?;
        Ok(())
    }

    /// Terminate the Worker and release all resources.
    #[wasm_bindgen(js_name = "terminate")]
    pub fn terminate(&self) {
        self.worker.terminate();
    }
}

// ===========================================================================
// Main-Thread Chunked Processing (Fallback)
// ===========================================================================

/// Process point cloud data in chunks on the main thread.
///
/// This is a fallback for environments where Web Workers are not available
/// (e.g. missing COOP/COEP headers). It processes the data in chunks and
/// yields to the main thread between chunks using `setTimeout(0)`.
///
/// The pipeline runs Octree build + Tileset generation. Since the actual
/// processing is synchronous in WASM, this function splits the work into
/// conceptual phases and calls `onChunk` after each phase.
///
/// # Arguments
/// * `positions` — `Float32Array` of `[x, y, z, ...]`.
/// * `colors` — Optional `Uint8Array` of `[r, g, b, ...]`.
/// * `max_points_per_node` — Max points per octree leaf (default: 50,000).
/// * `max_depth` — Max tree depth (default: 21).
/// * `on_chunk` — Callback `(phase: string, done: number, total: number)`.
///
/// # Returns
/// A `Promise` that resolves with the `TilesetResult` (as JSON string).
#[wasm_bindgen(js_name = "processChunked")]
pub fn process_chunked(
    positions: &[f32],
    colors: Option<Vec<u8>>,
    max_points_per_node: Option<u32>,
    max_depth: Option<u32>,
    on_chunk: &js_sys::Function,
) -> js_sys::Promise {
    let max_pts = max_points_per_node.unwrap_or(50_000);
    let max_d = max_depth.unwrap_or(21);
    let mut buf = positions.to_vec();
    let colors_clone = colors.clone();
    let on_chunk_clone = on_chunk.clone();

    let future = wasm_bindgen_futures::future_to_promise(async move {
        let this = JsValue::NULL;

        // Phase 1: Build octree (this is the expensive part)
        let _ = on_chunk_clone.call3(
            &this,
            &"octree".into(),
            &JsValue::from(0u32),
            &JsValue::from(2u32),
        );

        // Yield to main thread before heavy work
        yield_to_main_thread().await;

        let octree = crate::octree::Octree::build(&mut buf, max_pts, max_d);

        let _ = on_chunk_clone.call3(
            &this,
            &"tileset".into(),
            &JsValue::from(1u32),
            &JsValue::from(2u32),
        );

        // Yield again before tileset generation
        yield_to_main_thread().await;

        let result = crate::pnts::generate_tileset(&octree, &buf, colors_clone.as_deref())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Build result object
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &"tilesetJson".into(),
            &JsValue::from_str(result.tileset_json()),
        )?;
        js_sys::Reflect::set(
            &obj,
            &"tileCount".into(),
            &JsValue::from(result.tile_count()),
        )?;
        js_sys::Reflect::set(
            &obj,
            &"totalBytes".into(),
            &JsValue::from(result.total_bytes() as u32),
        )?;

        let _ = on_chunk_clone.call3(
            &this,
            &"done".into(),
            &JsValue::from(2u32),
            &JsValue::from(2u32),
        );

        Ok(obj.into())
    });

    future
}

/// Yield to the main thread by waiting for a resolved Promise.
async fn yield_to_main_thread() {
    let promise = js_sys::Promise::resolve(&JsValue::NULL);
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_script_generation() {
        let script = generate_worker_script();
        assert!(
            script.contains("'use strict'"),
            "Script should start with strict mode"
        );
        assert!(
            script.contains("self.onmessage"),
            "Script should define onmessage handler"
        );
        assert!(
            script.contains("type === 'init'"),
            "Script should handle init message"
        );
        assert!(
            script.contains("type === 'process'"),
            "Script should handle process message"
        );
        assert!(
            script.contains("type === 'cancel'"),
            "Script should handle cancel message"
        );
        assert!(
            script.contains("buildOctree"),
            "Script should call buildOctree"
        );
        assert!(
            script.contains("generateTileset"),
            "Script should call generateTileset"
        );
        assert!(
            script.contains("cancelled"),
            "Script should support cancellation"
        );
        assert!(
            script.contains("tileBuffers"),
            "Script should transfer ArrayBuffers"
        );
    }

    #[test]
    fn test_worker_script_is_valid_js_syntax() {
        let script = generate_worker_script();
        // Check balanced braces
        let open_braces = script.chars().filter(|&c| c == '{').count();
        let close_braces = script.chars().filter(|&c| c == '}').count();
        assert_eq!(
            open_braces, close_braces,
            "Unbalanced braces in worker script"
        );

        let open_parens = script.chars().filter(|&c| c == '(').count();
        let close_parens = script.chars().filter(|&c| c == ')').count();
        assert_eq!(
            open_parens, close_parens,
            "Unbalanced parentheses in worker script"
        );
    }

    #[test]
    fn test_worker_options_defaults() {
        let opts = WorkerOptions::new();
        assert_eq!(opts.max_points_per_node(), 50_000);
        assert_eq!(opts.max_depth(), 21);
    }

    #[test]
    fn test_worker_options_setters() {
        let mut opts = WorkerOptions::new();
        opts.set_max_points_per_node(10000);
        opts.set_max_depth(10);
        assert_eq!(opts.max_points_per_node(), 10000);
        assert_eq!(opts.max_depth(), 10);
    }

    #[test]
    fn test_worker_options_default_trait() {
        let opts = WorkerOptions::default();
        assert_eq!(opts.max_points_per_node(), 50_000);
        assert_eq!(opts.max_depth(), 21);
    }

    #[test]
    fn test_worker_script_contains_all_phases() {
        let script = generate_worker_script();
        // Verify the script reports progress for both phases
        assert!(
            script.contains("octree"),
            "Script should mention octree phase"
        );
        assert!(
            script.contains("tileset"),
            "Script should mention tileset phase"
        );
        // Verify error handling
        assert!(script.contains("error"), "Script should handle errors");
        // Verify postMessage types
        assert!(
            script.contains("'ready'"),
            "Script should post ready message"
        );
        assert!(
            script.contains("'complete'"),
            "Script should post complete message"
        );
        assert!(
            script.contains("'cancelled'"),
            "Script should post cancelled message"
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_supports_worker_returns_bool() {
        let _result = supports_worker();
    }

    #[test]
    fn test_worker_script_terrain_support() {
        let script = generate_worker_script();
        assert!(
            script.contains("type === 'terrain'"),
            "Script should handle terrain message type"
        );
        assert!(
            script.contains("parseGeotiff"),
            "Script should import parseGeotiff"
        );
        assert!(
            script.contains("terrainToGlb"),
            "Script should import terrainToGlb"
        );
        assert!(
            script.contains("applyTerrainColorRamp"),
            "Script should import applyTerrainColorRamp"
        );
        assert!(
            script.contains("hillshade"),
            "Script should import hillshade"
        );
    }

    #[test]
    fn test_worker_script_terrain_progress_phases() {
        let script = generate_worker_script();
        assert!(
            script.contains("'parse'"),
            "Script should have parse progress phase"
        );
        assert!(
            script.contains("'style'"),
            "Script should have style progress phase"
        );
        assert!(
            script.contains("'mesh'"),
            "Script should have mesh progress phase"
        );
        assert!(
            script.contains("'terrain'"),
            "Script should report terrain error stage"
        );
    }

    #[test]
    fn test_worker_script_terrain_result_fields() {
        let script = generate_worker_script();
        assert!(
            script.contains("glb"),
            "Terrain result should contain GLB data"
        );
        assert!(
            script.contains("colorRgba"),
            "Terrain result should contain color RGBA data"
        );
        assert!(
            script.contains("hillshadeData"),
            "Terrain result should contain hillshade data"
        );
        assert!(
            script.contains("width"),
            "Terrain result should contain width"
        );
        assert!(
            script.contains("height"),
            "Terrain result should contain height"
        );
        assert!(
            script.contains("elevationCount"),
            "Terrain result should contain elevation count"
        );
    }
}
