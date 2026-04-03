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
}

export interface WorkerResponse {
  id: number;
  result: ExtractionResult;
}
