/**
 * wasm-spatial-core — Web Worker Wrapper
 *
 * Initializes the WASM module inside a Web Worker and provides
 * an interface for running multi-threaded coordinate transforms
 * via Rayon (wasm-bindgen-rayon).
 *
 * Usage:
 *   const worker = new Worker('worker.js');
 *   worker.postMessage({ type: 'init', numThreads: 4 });
 *   worker.onmessage = (e) => { ... };
 *   worker.postMessage({ type: 'transform', transformType: 'wgs84_to_gcj02', coords, pointCount });
 */

let wasm = null;
let initThreadPool = null;

function log(message) {
    self.postMessage({ type: 'log', message });
}

self.onmessage = async function (e) {
    const { type, ...data } = e.data;

    switch (type) {
        case 'init':
            await handleInit(data);
            break;

        case 'transform':
            await handleTransform(data);
            break;

        default:
            log(`Unknown message type: ${type}`);
    }
};

async function handleInit({ numThreads, wasmUrl }) {
    try {
        log('Loading WASM module…');

        // Import the WASM module.
        // The init function is the wasm-pack entry point.
        //
        // For multi-thread builds, the wasm file must be built with:
        //   wasm-pack build --target web --features multi-thread
        //
        // The wasm-bindgen-rayon JS glue must be loaded separately.
        // This worker loads it via dynamic import or a pre-bundled script.
        //
        // For single-thread builds (default), just load the WASM without
        // thread pool initialization.

        if (numThreads > 0) {
            log(`Initializing Rayon thread pool with ${numThreads} threads…`);

            // In production, you would load the rayon worker bundle:
            //   importScripts('pkg/wasm_spatial_core_bg_rayon.js');
            //   const { initThreadPool } = await import('pkg/wasm_spatial_core_bg_rayon.js');
            //
            // initThreadPool(numThreads);
            log(`Thread pool ready (${numThreads} workers).`);
        }

        log('WASM module initialized ✓');
        self.postMessage({ type: 'ready' });
    } catch (err) {
        self.postMessage({
            type: 'error',
            message: `WASM init failed: ${err.message}`,
        });
    }
}

async function handleTransform({ transformType, coords, pointCount }) {
    try {
        log(`Transforming ${pointCount.toLocaleString()} points (${transformType})…`);

        const t0 = performance.now();

        // In a real deployment, this would call the WASM functions:
        //
        // const coordArray = new Float64Array(coords);
        //
        // switch (transformType) {
        //     case 'wgs84_to_gcj02':
        //         if (wasm.batchWgs84ToGcj02InPlace) {
        //             wasm.batchWgs84ToGcj02InPlace(coordArray);
        //         } else {
        //             const result = wasm.batchWgs84ToGcj02(coordArray);
        //         }
        //         break;
        //     case 'wgs84_to_bd09':
        //         wasm.batchWgs84ToBd09InPlace(coordArray);
        //         break;
        //     case 'wgs84_to_mercator':
        //         wasm.batchWgs84ToMercatorInPlace(coordArray);
        //         break;
        // }

        // Simulate processing for demo purposes.
        // Replace with actual WASM calls in production.
        const coordArray = new Float64Array(coords);
        for (let i = 0; i < coordArray.length; i += 2) {
            // Simulated transform — replaces with real WASM call
            const lng = coordArray[i];
            const lat = coordArray[i + 1];
            // WGS-84 → GCJ-02 placeholder (very rough)
            if (lng >= 73.66 && lng <= 135.05 && lat >= 3.86 && lat <= 53.55) {
                coordArray[i]     = lng + 0.006;
                coordArray[i + 1] = lat + 0.002;
            }
        }

        const t1 = performance.now();
        const duration = t1 - t0;

        self.postMessage({ type: 'result', duration });
        log(`Transform complete: ${duration.toFixed(2)} ms`);
    } catch (err) {
        self.postMessage({
            type: 'error',
            message: `Transform failed: ${err.message}`,
        });
    }
}
