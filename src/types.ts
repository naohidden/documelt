/** WASM側から返る生データ */
export interface WasmExtractionResult {
  texts: string[];
  success: boolean;
  error: string | null;
  pages: number;
}

/** 利用者に返す最終結果 */
export interface ExtractionResult {
  texts: string[];
  success: boolean;
  error: string | null;
  meta: ExtractionMeta;
  /** `options.format === 'markdown'` を指定したときのみ含まれる結合済み Markdown */
  markdown?: string;
}

/** 抽出時のオプション */
export interface ExtractOptions {
  /** 出力フォーマット。`'markdown'` を指定すると result.markdown に整形済み文字列を返す */
  format?: 'text' | 'markdown';
}

export interface ExtractionMeta {
  filename: string;
  extension: SupportedFormat;
  size: number;
  pages: number;
  characters: number;
  time: number;
}

export type SupportedFormat = 'pdf' | 'docx' | 'xlsx' | 'pptx' | 'txt';

export const SUPPORTED_FORMATS: SupportedFormat[] = ['pdf', 'docx', 'xlsx', 'pptx', 'txt'];

// Worker通信用メッセージ型
export interface WorkerRequest {
  id: number;
  data: Uint8Array;
  filename: string;
  extension: SupportedFormat;
  options?: ExtractOptions;
}

export interface WorkerResponse {
  id: number;
  result: ExtractionResult;
}
