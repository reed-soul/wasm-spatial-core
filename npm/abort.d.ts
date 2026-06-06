/** AbortSignal helpers for cancellable WASM pipelines. */

export declare function createAbortChecker(
  signal?: AbortSignal | null,
): () => boolean;

export declare function linkAbortSignalToWorker(
  worker: { cancel: () => void },
  signal?: AbortSignal | null,
): () => void;

export declare function runWithAbortSignal<T>(
  signal: AbortSignal | undefined,
  fn: (shouldAbort: () => boolean) => T | Promise<T>,
): Promise<T>;
