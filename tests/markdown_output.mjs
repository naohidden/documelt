import { readFile, writeFile } from 'fs/promises';
import { initWithBytes, extract } from '../dist/index.js';

const wasmBytes = await readFile(new URL('../pkg/documelt_bg.wasm', import.meta.url));
initWithBytes(wasmBytes);

const files = [
  { path: '../samples/sample.txt', ext: 'txt' },
  { path: '../samples/sample.pdf', ext: 'pdf' },
  { path: '../samples/sample.docx', ext: 'docx' },
  { path: '../samples/sample.xlsx', ext: 'xlsx' },
  { path: '../samples/sample.pptx', ext: 'pptx' },
  { path: '../samples/sample_rich.docx', ext: 'docx', name: 'sample_rich' },
];

let failed = 0;

for (const { path, ext, name } of files) {
  const data = await readFile(new URL(path, import.meta.url));
  const filename = `${name ?? 'sample'}.${ext}`;
  const result = await extract(new Uint8Array(data), ext, filename, { format: 'markdown' });

  if (!result.success) {
    console.error(`${filename}: FAILED (${result.error ?? 'unknown error'})`);
    failed++;
    continue;
  }

  // format: 'markdown' を指定すると texts の各要素が Markdown になる
  const markdown = result.texts.join('\n\n');
  const outPath = new URL(`../samples/extracted_${name ?? ext}.md`, import.meta.url);
  await writeFile(outPath, markdown, 'utf-8');

  console.log(`${filename}: ${result.texts.length} page(s) | ${markdown.length} chars`);
  console.log(markdown.slice(0, 160).replace(/\n/g, '\\n'));
  console.log('---');
}

if (failed > 0) {
  console.error(`\n${failed} file(s) failed`);
  process.exit(1);
}
