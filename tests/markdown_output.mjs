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
];

for (const { path, ext } of files) {
  const data = await readFile(new URL(path, import.meta.url));
  const filename = `sample.${ext}`;
  const result = await extract(new Uint8Array(data), ext, filename, { format: 'markdown' });

  if (result.success && result.markdown) {
    const outPath = new URL(`../samples/extracted_${ext}.md`, import.meta.url);
    await writeFile(outPath, result.markdown, 'utf-8');
    console.log(`${ext}: saved | ${result.markdown.length} chars | preview:`);
    console.log(result.markdown.slice(0, 200).replace(/\n/g, '\\n'));
    console.log('---');
  } else {
    console.log(`${ext}: FAILED (${result.error ?? 'no markdown'})`);
  }
}
