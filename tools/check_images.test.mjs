import test from 'node:test';
import assert from 'node:assert/strict';
import { probe, inspect, readBriefs, missingList } from './check_images.mjs';

function png(width, height, colorType) {
  const b = Buffer.alloc(33);
  b.writeUInt32BE(0x89504e47, 0);
  b.writeUInt32BE(0x0d0a1a0a, 4);
  b.writeUInt32BE(13, 8);
  b.write('IHDR', 12, 'ascii');
  b.writeUInt32BE(width, 16);
  b.writeUInt32BE(height, 20);
  b[24] = 8;
  b[25] = colorType;
  return b;
}

function jpeg(width, height) {
  // SOI, APP0（長さ16）, SOF0
  const app0 = Buffer.from([0xff, 0xe0, 0x00, 0x10, ...Buffer.alloc(14)]);
  const sof = Buffer.from([0xff, 0xc0, 0x00, 0x11, 0x08, height >> 8, height & 255, width >> 8, width & 255, 0x03, ...Buffer.alloc(9)]);
  return Buffer.concat([Buffer.from([0xff, 0xd8]), app0, sof]);
}

test('reads PNG and JPEG sizes', () => {
  assert.deepEqual(probe(png(512, 512, 6)), { format: 'png', width: 512, height: 512, alpha: true });
  assert.deepEqual(probe(png(10, 20, 2)), { format: 'png', width: 10, height: 20, alpha: false });
  assert.deepEqual(probe(jpeg(1280, 720)), { format: 'jpg', width: 1280, height: 720, alpha: false });
  assert.equal(probe(Buffer.from('not an image')), null);
});

test('reports size, format and transparency problems', () => {
  const portrait = { width: 512, height: 512, format: 'jpg', alpha: false };
  const treasure = { width: 512, height: 512, format: 'png', alpha: true };
  assert.deepEqual(inspect(jpeg(512, 512), portrait), []);
  assert.equal(inspect(jpeg(500, 512), portrait).length, 1);
  assert.equal(inspect(png(512, 512, 6), portrait).length, 1);
  assert.deepEqual(inspect(png(512, 512, 6), treasure), []);
  assert.match(inspect(png(512, 512, 2), treasure)[0], /透過/);
  assert.equal(inspect(Buffer.from('x'), portrait).length, 1);
});

test('lists only missing images with their briefs', () => {
  const manifest = { categories: { portrait: { title: '人物の肖像', width: 512, height: 512, format: 'jpg', alpha: false } } };
  const results = [
    { path: 'portraits/a.jpg', category: 'portrait', title: 'A', era: '時代1', status: 'ok' },
    { path: 'portraits/b.jpg', category: 'portrait', title: 'B', era: '時代1', status: 'missing' },
    { path: 'portraits/c.jpg', category: 'portrait', title: 'C', era: '時代2', status: 'missing' },
  ];
  const briefs = readBriefs('| `portraits/b.jpg` | 人物の肖像：B | Bの説明 |\r\n| `portraits/c.jpg` | 人物の肖像：C | Cの説明 |');
  const md = missingList({ manifest, results }, briefs);
  assert.match(md, /まだ無い画像：2 枚（必要な画像 3 枚のうち）/);
  assert.doesNotMatch(md, /portraits\/a\.jpg/);
  assert.match(md, /## 時代1（1 枚）/);
  assert.match(md, /\| `portraits\/c\.jpg` \| 人物の肖像：C \| Cの説明 \|/);
});
