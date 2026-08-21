/** インライン要素。ひと続きの装飾範囲を表す */
export interface Run {
  text: string;
  bold?: boolean;
  italic?: boolean;
  strike?: boolean;
  code?: boolean;
  /** ハイライト色 */
  highlight?: string;
  /** ハイパーリンクの URL */
  link?: string;
}

export interface ListItemBlock {
  level: number;
  runs: Run[];
}

/** ブロック要素。ページ(スライド/シート)は Block[] で表す */
export type Block =
  | { type: 'heading'; level: number; runs: Run[] }
  | { type: 'para'; runs: Run[] }
  | { type: 'list'; ordered: boolean; items: ListItemBlock[] }
  /** rows[行][列] = セル内の Run 列 */
  | { type: 'table'; rows: Run[][][] }
  | { type: 'code'; text: string };

/** WASM側から返る生データ */
export interface WasmExtractionResult {
  texts: string[];
  /** ページ単位の構造化ブロック。Markdown 整形に使う */
  blocks: Block[][];
  success: boolean;
  error: string | null;
  pages: number;
}

/** 利用者に返す最終結果 */
export interface ExtractionResult {
  /**
   * ページ(スライド/シート)単位の抽出結果。
   * `options.format === 'markdown'` のときは各要素が Markdown 文字列になる。
   */
  texts: string[];
  success: boolean;
  error: string | null;
  meta: ExtractionMeta;
}

/** 抽出時のオプション */
export interface ExtractOptions {
  /** 出力フォーマット。`'markdown'` を指定すると texts が Markdown 文字列になる（デフォルト `'text'`） */
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
