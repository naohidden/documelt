import type { SupportedFormat } from '../types.js';

/**
 * 抽出済みテキスト配列を Markdown 文字列に整形する。
 *
 * - PDF      : `## Page N` 見出し付き
 * - PPTX     : `## Slide N` 見出し付き
 * - XLSX     : 各シートを `## SheetName` + Markdown table に変換
 *              (calamine のタブ区切り行を読み取り、1行目をヘッダ扱い)
 * - DOCX/TXT : テキストをそのまま結合
 */
export function toMarkdown(texts: string[], extension: SupportedFormat, filename: string): string {
  if (texts.length === 0) return filename ? `# ${filename}\n` : '';

  const title = filename ? `# ${filename}\n\n` : '';

  switch (extension) {
    case 'pdf':
      return title + texts.map((t, i) => `## Page ${i + 1}\n\n${t.trim()}`).join('\n\n');
    case 'pptx':
      return title + texts.map((t, i) => `## Slide ${i + 1}\n\n${t.trim()}`).join('\n\n');
    case 'xlsx':
      return title + texts.map(sheetToMarkdown).join('\n\n');
    case 'docx':
    case 'txt':
    default:
      return title + texts.join('\n\n').trim();
  }
}

/**
 * XLSX のシート出力 (`## SheetName\n` + タブ区切り行) を
 * `## SheetName` + Markdown table に変換する。
 */
function sheetToMarkdown(sheetText: string): string {
  const lines = sheetText.split('\n');
  const heading = lines[0]?.startsWith('## ') ? lines[0] : `## ${lines[0] ?? ''}`;
  const bodyLines = (lines[0]?.startsWith('## ') ? lines.slice(1) : lines)
    .filter((l) => l.length > 0);

  if (bodyLines.length === 0) return heading;

  const rows = bodyLines.map((l) => l.split('\t').map(escapeCell));
  const cols = Math.max(...rows.map((r) => r.length));
  const padded = rows.map((r) => {
    const out = r.slice();
    while (out.length < cols) out.push('');
    return out;
  });

  const header = padded[0];
  const sep = new Array(cols).fill('---');
  const dataRows = padded.slice(1);

  const table = [
    `| ${header.join(' | ')} |`,
    `| ${sep.join(' | ')} |`,
    ...dataRows.map((r) => `| ${r.join(' | ')} |`),
  ].join('\n');

  return `${heading}\n\n${table}`;
}

function escapeCell(s: string): string {
  return s.replace(/\\/g, '\\\\').replace(/\|/g, '\\|').replace(/\r?\n/g, ' ');
}
