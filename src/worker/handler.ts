import initWasm, { extract as wasmExtract } from '../../pkg/documelt.js';
import type { WorkerRequest, WorkerResponse, WasmExtractionResult, ExtractionResult } from '../types.js';
import { renderMarkdown } from '../core/markdown.js';

let initialized = false;

self.onmessage = async (e: MessageEvent<WorkerRequest>) => {
  const { id, data, filename, extension, options } = e.data;

  try {
    if (!initialized) {
      await initWasm();
      initialized = true;
    }

    const start = performance.now();
    const wasm = wasmExtract(data, extension) as WasmExtractionResult;
    const time = Math.round(performance.now() - start) / 1000;

    const texts =
      options?.format === 'markdown' && wasm.success
        ? wasm.blocks.map(renderMarkdown)
        : wasm.texts;

    const result: ExtractionResult = {
      texts,
      success: wasm.success,
      error: wasm.error,
      meta: {
        filename,
        extension,
        size: data.byteLength,
        pages: wasm.pages,
        characters: texts.join('\n').length,
        time,
      },
    };
    const response: WorkerResponse = { id, result };
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
