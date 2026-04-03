import initWasm, { extract as wasmExtract } from '../../pkg/documelt.js';
import type { WorkerRequest, WorkerResponse, WasmExtractionResult } from '../types.js';

let initialized = false;

self.onmessage = async (e: MessageEvent<WorkerRequest>) => {
  const { id, data, filename, extension } = e.data;

  try {
    if (!initialized) {
      await initWasm();
      initialized = true;
    }

    const start = performance.now();
    const wasm = wasmExtract(data, extension) as WasmExtractionResult;
    const time = Math.round(performance.now() - start) / 1000;

    const joined = wasm.texts.join('\n');
    const response: WorkerResponse = {
      id,
      result: {
        texts: wasm.texts,
        success: wasm.success,
        error: wasm.error,
        meta: {
          filename,
          extension,
          size: data.byteLength,
          pages: wasm.pages,
          characters: joined.length,
          time,
        },
      },
    };
    self.postMessage(response);
  } catch (err) {
    const response: WorkerResponse = {
      id,
      result: {
        texts: [],
        success: false,
        error: err instanceof Error ? err.message : String(err),
        meta: {
          filename,
          extension,
          size: data.byteLength,
          pages: 0,
          characters: 0,
          time: 0,
        },
      },
    };
    self.postMessage(response);
  }
};
