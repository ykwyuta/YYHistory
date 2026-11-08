import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, rm, stat } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { packageWasm } from './package_wasm.mjs';

async function fixture(t, bytes) {
  const base = path.resolve(os.tmpdir());
  const directory = await mkdtemp(path.join(base, 'yyhistory-wasm-'));
  t.after(async () => {
    // Verify the exact temporary target before recursive cleanup on Windows.
    assert.equal(path.dirname(path.resolve(directory)), base);
    assert.ok(path.basename(directory).startsWith('yyhistory-wasm-'));
    await rm(directory, { recursive: true, force: true });
  });
  const file = path.join(directory, 'yyhistory.wasm');
  await writeFile(file, bytes);
  return { directory, file };
}

test('chunked assets reconstruct the exact original bytes within the size limit', async t => {
  const bytes = Buffer.alloc(3101, 42);
  bytes.set([0, 97, 115, 109, 1, 0, 0, 0]);
  const { directory, file } = await fixture(t, bytes);
  const manifest = await packageWasm(file, 1024);
  assert.equal(manifest.parts.length, 4);
  const parts = await Promise.all(manifest.parts.map(part => readFile(path.join(directory, part))));
  assert.ok(parts.every(part => part.length <= 1024));
  assert.deepEqual(Buffer.concat(parts), bytes);
  assert.equal(manifest.bytes, bytes.length);
  assert.deepEqual(JSON.parse(await readFile(path.join(directory, 'wasm-manifest.json'), 'utf8')), manifest);
  await assert.rejects(stat(file), { code: 'ENOENT' });
});

test('small builds keep the streaming-compatible single wasm file', async t => {
  const bytes = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]);
  const { file } = await fixture(t, bytes);
  assert.deepEqual((await packageWasm(file, 1024)).parts, ['yyhistory.wasm']);
  assert.deepEqual(await readFile(file), bytes);
});
