// 必要な画像（assets/images/manifest.json）がそろっているかを調べる。
//   node tools/check_images.mjs            抜けを報告（抜けがあっても終了コード0）
//   node tools/check_images.mjs --strict   抜けが1枚でもあれば終了コード1
//   node tools/check_images.mjs --report   output/images-report.md にチェックリストを書く
//   --category portrait|event|treasure|era|map|ui   --era 平安京   で絞り込み
// 置いてある画像の形式・大きさ・透過が規格と違うとき、一覧に無い余分な画像があるときは、常に終了コード1。
import { readFileSync, existsSync, readdirSync, statSync, mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const IMAGE_EXT = new Set(['.png', '.jpg', '.jpeg', '.webp', '.gif']);

/** PNG / JPEG の形式・幅・高さ・透過の有無を読む（読めなければ null） */
export function probe(buf) {
  if (buf.length >= 33 && buf.readUInt32BE(0) === 0x89504e47 && buf.toString('ascii', 12, 16) === 'IHDR') {
    const colorType = buf[25];
    const alpha = colorType === 4 || colorType === 6 || buf.includes(Buffer.from('tRNS'));
    return { format: 'png', width: buf.readUInt32BE(16), height: buf.readUInt32BE(20), alpha };
  }
  if (buf.length >= 4 && buf[0] === 0xff && buf[1] === 0xd8) {
    let at = 2;
    while (at + 9 < buf.length) {
      if (buf[at] !== 0xff) { at++; continue; }
      const marker = buf[at + 1];
      if (marker === 0xd8 || marker === 0x01 || (marker >= 0xd0 && marker <= 0xd7)) { at += 2; continue; }
      const len = buf.readUInt16BE(at + 2);
      // SOF0〜SOF15（DHT=C4, JPG=C8, DAC=CC を除く）
      if (marker >= 0xc0 && marker <= 0xcf && ![0xc4, 0xc8, 0xcc].includes(marker)) {
        return { format: 'jpg', width: buf.readUInt16BE(at + 7), height: buf.readUInt16BE(at + 5), alpha: false };
      }
      at += 2 + len;
    }
  }
  return null;
}

/** 一枚の画像を規格と照らし合わせる。問題の説明の配列を返す（空なら合格） */
export function inspect(buf, spec) {
  const info = probe(buf);
  if (!info) return [`${spec.format.toUpperCase()} として読めない`];
  const problems = [];
  if (info.format !== spec.format) problems.push(`形式が ${info.format.toUpperCase()}（${spec.format.toUpperCase()} にする）`);
  if (info.width !== spec.width || info.height !== spec.height) {
    problems.push(`大きさが ${info.width}×${info.height}（${spec.width}×${spec.height} にする）`);
  }
  if (spec.alpha && !info.alpha) problems.push('背景が透過になっていない');
  return problems;
}

function walk(dir, base = dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((name) => {
    const full = path.join(dir, name);
    return statSync(full).isDirectory() ? walk(full, base) : [path.relative(base, full).split(path.sep).join('/')];
  });
}

export function check(root, { category = null, era = null } = {}) {
  const dir = path.join(root, 'assets/images');
  const manifest = JSON.parse(readFileSync(path.join(dir, 'manifest.json'), 'utf8'));
  const wanted = manifest.images.filter((i) => (!category || i.category === category) && (!era || i.era === era));
  const results = wanted.map((img) => {
    const spec = manifest.categories[img.category];
    const file = path.join(dir, img.path);
    if (!existsSync(file)) return { ...img, status: 'missing', problems: [] };
    const problems = inspect(readFileSync(file), spec);
    return { ...img, status: problems.length ? 'invalid' : 'ok', problems };
  });
  const known = new Set(manifest.images.map((i) => i.path));
  const extra = category || era ? [] : walk(dir).filter((p) => IMAGE_EXT.has(path.extname(p).toLowerCase()) && !known.has(p));
  return { manifest, results, extra };
}

function report({ manifest, results, extra }) {
  const lines = ['# 画像のチェックリスト', '', `生成: node tools/check_images.mjs --report（${new Date().toISOString().slice(0, 10)}）`, ''];
  const done = results.filter((r) => r.status === 'ok').length;
  lines.push(`そろっている画像 ${done} / ${results.length}`, '');
  const eras = [...new Set(results.map((r) => r.era))];
  for (const era of eras) {
    lines.push(`## ${era || '共通'}`, '');
    for (const r of results.filter((x) => x.era === era)) {
      const mark = r.status === 'ok' ? '[x]' : '[ ]';
      const note = r.status === 'invalid' ? ` ⚠ ${r.problems.join('、')}` : '';
      lines.push(`- ${mark} \`${r.path}\` ${manifest.categories[r.category].title}：${r.title}${note}`);
    }
    lines.push('');
  }
  if (extra.length) lines.push('## 一覧に無い画像', '', ...extra.map((p) => `- \`${p}\``), '');
  return lines.join('\n');
}

function main() {
  const args = process.argv.slice(2);
  const opt = (name) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : null; };
  const root = fileURLToPath(new URL('../', import.meta.url));
  const result = check(root, { category: opt('--category'), era: opt('--era') });
  const { manifest, results, extra } = result;
  const missing = results.filter((r) => r.status === 'missing');
  const invalid = results.filter((r) => r.status === 'invalid');
  console.log(`画像: ${results.length - missing.length - invalid.length} / ${results.length} 枚そろっています（assets/images/）`);
  for (const [key, c] of Object.entries(manifest.categories)) {
    const all = results.filter((r) => r.category === key);
    if (!all.length) continue;
    const ok = all.filter((r) => r.status === 'ok').length;
    console.log(`  ${c.title.padEnd(8, '　')} ${String(ok).padStart(3)} / ${String(all.length).padEnd(3)} ${c.width}×${c.height} ${c.format.toUpperCase()}${c.alpha ? '（透過）' : ''}`);
  }
  if (invalid.length) {
    console.log('\n規格と違う画像:');
    for (const r of invalid) console.log(`  ✗ ${r.path}：${r.problems.join('、')}`);
  }
  if (extra.length) {
    console.log('\n一覧に無い画像（パスの間違い？）:');
    for (const p of extra) console.log(`  ? ${p}`);
  }
  if (missing.length) {
    console.log(`\nまだ無い画像 ${missing.length} 枚:`);
    let era = undefined;
    for (const r of missing.slice(0, args.includes('--all') ? missing.length : 30)) {
      if (r.era !== era) { era = r.era; console.log(`  【${era || '共通'}】`); }
      console.log(`    - ${r.path}　${manifest.categories[r.category].title}：${r.title}`);
    }
    if (!args.includes('--all') && missing.length > 30) console.log(`    …ほか ${missing.length - 30} 枚（--all ですべて表示。説明は docs/images.md）`);
  }
  if (args.includes('--report')) {
    mkdirSync(path.join(root, 'output'), { recursive: true });
    writeFileSync(path.join(root, 'output/images-report.md'), report(result));
    console.log('\noutput/images-report.md に書き出しました。');
  }
  if (invalid.length || extra.length) process.exit(1);
  if (args.includes('--strict') && missing.length) process.exit(1);
}

if (import.meta.url === pathToFileURL(process.argv[1] || '').href) main();
