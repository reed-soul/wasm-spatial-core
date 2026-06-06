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
export function createAbortChecker(signal?: AbortSignal | null): () => boolean {
  return () => Boolean(signal?.aborted);
}

/**
 * Wire a standard AbortSignal to a WorkerHandle.cancel().
 * Returns a dispose function to remove the listener.
 */
export function linkAbortSignalToWorker(
  worker: { cancel: () => void },
  signal?: AbortSignal | null,
): () => void {
  if (!signal) {
    return () => {};
  }
  const onAbort = () => worker.cancel();
  signal.addEventListener("abort", onAbort);
  return () => signal.removeEventListener("abort", onAbort);
}

/**
 * Run an async WASM/Worker job with AbortSignal support.
 * Rejects with DOMException `AbortError` when aborted.
 */
export async function runWithAbortSignal<T>(
  signal: AbortSignal | undefined,
  fn: (shouldAbort: () => boolean) => T | Promise<T>,
): Promise<T> {
  const shouldAbort = createAbortChecker(signal);
  if (shouldAbort()) {
    throw new DOMException("Aborted", "AbortError");
  }
  const result = await fn(shouldAbort);
  if (shouldAbort()) {
    throw new DOMException("Aborted", "AbortError");
  }
  return result;
}
