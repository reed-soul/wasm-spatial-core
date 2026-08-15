/**
 * AbortSignal adapter for cancellable WASM pipelines (Wave 1.2).
 *
 * @example
 * ```ts
 * import { createAbortChecker, linkAbortSignalToWorker } from "wasm-spatial-core/abort";
 *
 * const controller = new AbortController();
 * const shouldAbort = createAbortChecker(controller.signal);
 *
 * // WASM parse with abort callback
 * core.parseLasPointsWithProgressAndAbort(bytes, onProgress, shouldAbort);
 *
 * // Worker: auto-cancel on signal
 * linkAbortSignalToWorker(worker, controller.signal);
 * controller.abort();
 * ```
 */
/** Returns a function suitable for WASM `shouldAbort` callbacks. */
export declare function createAbortChecker(signal?: AbortSignal | null): () => boolean;
/**
 * Wire a standard AbortSignal to a WorkerHandle.cancel().
 * Returns a dispose function to remove the listener.
 */
export declare function linkAbortSignalToWorker(worker: {
    cancel: () => void;
}, signal?: AbortSignal | null): () => void;
/**
 * Run an async WASM/Worker job with AbortSignal support.
 * Rejects with DOMException `AbortError` when aborted.
 */
export declare function runWithAbortSignal<T>(signal: AbortSignal | undefined, fn: (shouldAbort: () => boolean) => T | Promise<T>): Promise<T>;
