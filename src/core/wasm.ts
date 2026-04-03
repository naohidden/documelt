import initWasm, { initSync, extract as wasmExtract } from '../../pkg/documelt.js';
import type { WasmExtractionResult, SupportedFormat } from '../types.js';

let initialized = false;
let initPromise: Promise<void> | null = null;

/**
 * WASMモジュールを初期化（ブラウザ用・非同期）
 * 通常は自動初期化されるため呼ぶ必要はない
 * 初期化タイミングを制御したい場合に使用
 */
export async function init(wasmUrl?: string | URL): Promise<void> {
  if (initialized) return;
  if (initPromise) return initPromise;
  initPromise = initWasm(wasmUrl).then(() => { initialized = true; });
  return initPromise;
}

/**
 * WASMモジュールを初期化（Node.js用・同期）
 * BufferSourceを直接渡して初期化する
 */
export function initWithBytes(wasmBytes: BufferSource): void {
  if (initialized) return;
  initSync({ module: wasmBytes });
  initialized = true;
}

/**
 * WASM関数を呼び出す（未初期化なら自動初期化）
 */
export async function callExtract(data: Uint8Array, extension: SupportedFormat): Promise<WasmExtractionResult> {
  await init();
  return wasmExtract(data, extension) as WasmExtractionResult;
}
