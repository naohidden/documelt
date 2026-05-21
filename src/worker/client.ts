import type { ExtractionResult, ExtractOptions, SupportedFormat, WorkerRequest, WorkerResponse } from '../types.js';
import { SUPPORTED_FORMATS } from '../types.js';

/**
 * Worker経由でドキュメントテキスト抽出を行うクライアント
 * メインスレッドをブロックしない
 */
export class DocumeltWorker {
  private worker: Worker;
  private nextId = 0;
  private pending = new Map<number, {
    resolve: (result: ExtractionResult) => void;
    reject: (error: Error) => void;
  }>();

  constructor(worker: Worker) {
    this.worker = worker;
    this.worker.onmessage = (e: MessageEvent<WorkerResponse>) => {
      const { id, result } = e.data;
      const handler = this.pending.get(id);
      if (handler) {
        this.pending.delete(id);
        handler.resolve(result);
      }
    };
    this.worker.onerror = (e) => {
      for (const [id, handler] of this.pending) {
        handler.reject(new Error(e.message));
        this.pending.delete(id);
      }
    };
  }

  /**
   * バイナリデータからテキストを抽出
   */
  extract(
    data: Uint8Array,
    extension: SupportedFormat,
    filename: string = '',
    options: ExtractOptions = {},
  ): Promise<ExtractionResult> {
    return new Promise((resolve, reject) => {
      const id = this.nextId++;
      this.pending.set(id, { resolve, reject });
      const request: WorkerRequest = { id, data, filename, extension, options };
      this.worker.postMessage(request, [data.buffer]);
    });
  }

  /**
   * Fileオブジェクトからテキストを抽出
   */
  async extractFromFile(file: File, options: ExtractOptions = {}): Promise<ExtractionResult> {
    const extension = file.name.split('.').pop()?.toLowerCase() as SupportedFormat;
    if (!SUPPORTED_FORMATS.includes(extension)) {
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
    return this.extract(new Uint8Array(buffer), extension, file.name, options);
  }

  /**
   * Workerを終了
   */
  terminate(): void {
    this.worker.terminate();
    for (const [, handler] of this.pending) {
      handler.reject(new Error('Worker terminated'));
    }
    this.pending.clear();
  }
}
