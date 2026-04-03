import type { ExtractionResult, SupportedFormat } from '../types.js';
import { SUPPORTED_FORMATS } from '../types.js';
import { callExtract } from './wasm.js';

function getExtension(fileName: string): string {
  return fileName.split('.').pop()?.toLowerCase() ?? '';
}

/**
 * 対応フォーマットかどうかを判定
 */
export function isSupported(fileName: string): boolean {
  const extension = getExtension(fileName);
  return (SUPPORTED_FORMATS as string[]).includes(extension);
}

/**
 * バイナリデータからテキストを抽出
 */
export async function extract(data: Uint8Array, extension: SupportedFormat, filename: string = ''): Promise<ExtractionResult> {
  const start = performance.now();
  const wasm = await callExtract(data, extension);
  const time = Math.round(performance.now() - start) / 1000;

  const joined = wasm.texts.join('\n');
  return {
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
  };
}

/**
 * Fileオブジェクトからテキストを抽出（ブラウザ用）
 */
export async function extractFromFile(file: File): Promise<ExtractionResult> {
  const extension = getExtension(file.name) as SupportedFormat;
  if (!isSupported(file.name)) {
    return {
      texts: [],
      success: false,
      error: `Unsupported format: ${extension}`,
      meta: {
        filename: file.name,
        extension,
        size: file.size,
        pages: 0,
        characters: 0,
        time: 0,
      },
    };
  }
  const buffer = await file.arrayBuffer();
  return extract(new Uint8Array(buffer), extension, file.name);
}
