import { readFile, writeFile } from 'fs/promises';
import { initSync, extract } from '../pkg/documelt.js';

const wasmBytes = await readFile(new URL('../pkg/documelt_bg.wasm', import.meta.url));
initSync({ module: wasmBytes });

const files = [
  { path: '../samples/sample.txt', ext: 'txt' },
  { path: '../samples/sample.pdf', ext: 'pdf' },
  { path: '../samples/sample.docx', ext: 'docx' },
  { path: '../samples/sample.xlsx', ext: 'xlsx' },
  { path: '../samples/sample.pptx', ext: 'pptx' },
];

for (const { path, ext } of files) {
  const data = await readFile(new URL(path, import.meta.url));
  const start = performance.now();
  const result = extract(new Uint8Array(data), ext);
  const time = ((performance.now() - start) / 1000).toFixed(3);

  if (result.success) {
    const joined = result.texts.join('\n');
    const outPath = new URL(`../samples/extracted_${ext}.txt`, import.meta.url);
    await writeFile(outPath, joined, 'utf-8');

    const chars = joined.length;
    const lines = joined.split('\n').length;
    console.log(`${ext}: saved | ${result.texts.length} pages | ${chars} chars | ${lines} lines | ${time}s | ${data.length} bytes`);
  } else {
    console.log(`${ext}: no text extracted (${result.error})`);
  }
}
