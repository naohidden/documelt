import type { ExtractionResult, ExtractOptions, SupportedFormat } from '../types.js';
import { SUPPORTED_FORMATS } from '../types.js';
import { callExtract } from './wasm.js';
import { renderMarkdown } from './markdown.js';

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
export async function extract(
  data: Uint8Array,
  extension: SupportedFormat,
  filename: string = '',
  options: ExtractOptions = {},
): Promise<ExtractionResult> {
  const start = performance.now();
  const wasm = await callExtract(data, extension);
  const time = Math.round(performance.now() - start) / 1000;

  const texts =
    options.format === 'markdown' && wasm.success
      ? wasm.blocks.map(renderMarkdown)
      : wasm.texts;

  return {
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
}

/**
 * Fileオブジェクトからテキストを抽出（ブラウザ用）
 */
export async function extractFromFile(file: File, options: ExtractOptions = {}): Promise<ExtractionResult> {
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
  return extract(new Uint8Array(buffer), extension, file.name, options);
}
