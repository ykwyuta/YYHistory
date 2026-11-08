// Keep static assets below Cloudflare's 25 MiB per-file limit.
import { readFile, writeFile, mkdir, unlink } from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';

export const CHUNK_BYTES = 24 * 1024 * 1024;

export async function packageWasm(input, chunkBytes = CHUNK_BYTES) {
  if (!Number.isSafeInteger(chunkBytes) || chunkBytes < 1) throw new Error('Invalid chunk size');
  const data = await readFile(input);
  if (data.length < 8 || !data.subarray(0, 4).equals(Buffer.from([0, 97, 115, 109]))) {
    throw new Error('Expected a WebAssembly binary');
  }
  const directory = path.dirname(input);
  const hash = createHash('sha256').update(data).digest('hex').slice(0, 16);
  const parts = [];
  if (data.length <= chunkBytes) {
    parts.push(path.basename(input));
  } else {
    await mkdir(path.join(directory, 'wasm'), { recursive: true });
    for (let offset = 0, i = 0; offset < data.length; offset += chunkBytes, i++) {
      const name = `wasm/${hash}.${String(i).padStart(3, '0')}.bin`;
      await writeFile(path.join(directory, name), data.subarray(offset, offset + chunkBytes));
      parts.push(name);
    }
  }
  const manifest = { parts, bytes: data.length };
  await writeFile(path.join(directory, 'wasm-manifest.json'), JSON.stringify(manifest) + '\n');
  if (parts.length > 1) await unlink(input);
  return manifest;
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const result = await packageWasm(process.argv[2] || 'dist/yyhistory.wasm');
  console.log(`Packaged ${result.bytes} bytes in ${result.parts.length} static asset(s)`);
}
