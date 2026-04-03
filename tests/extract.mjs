import { readFile } from 'fs/promises';
import { initSync, extract } from '../pkg/documelt.js';

const wasmBytes = await readFile(new URL('../pkg/documelt_bg.wasm', import.meta.url));
initSync({ module: wasmBytes });

const files = [
  { path: '../samples/sample.txt', extension: 'txt' },
  { path: '../samples/sample.pdf', extension: 'pdf' },
  { path: '../samples/sample.docx', extension: 'docx' },
  { path: '../samples/sample.xlsx', extension: 'xlsx' },
  { path: '../samples/sample.pptx', extension: 'pptx' },
];

for (const { path, extension } of files) {
  console.log(`\n=== ${extension.toUpperCase()} ===`);
  try {
    const data = await readFile(new URL(path, import.meta.url));
    const result = extract(new Uint8Array(data), extension);
    console.log(`success: ${result.success}`);
    console.log(`pages: ${result.pages}`);
    console.log(`error: ${result.error || 'none'}`);
    if (result.texts.length > 0) {
      const joined = result.texts.join('\n');
      const preview = joined.substring(0, 200);
      console.log(`text (first 200 chars): ${preview}`);
      console.log(`total length: ${joined.length}`);
    } else {
      console.log('texts: empty');
    }
  } catch (e) {
    console.log(`FAILED: ${e.message}`);
  }
}
