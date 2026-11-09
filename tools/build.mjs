// Cross-platform browser build; no npm dependencies required.
import { spawnSync } from 'node:child_process';
import { mkdirSync, copyFileSync, writeFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root=fileURLToPath(new URL('../',import.meta.url));
function checkResult(result, command) {
  if (result.error) throw new Error(`Could not start ${command}: ${result.error.message}`, { cause: result.error });
  if (result.signal) throw new Error(`${command} was terminated by ${result.signal}`);
  if (result.status !== 0) process.exit(result.status || 1);
}
// Linux build services may only provide Node. The shell build also installs Rust
// and its WASM target; Windows keeps the native Node build without requiring Bash.
if (process.platform !== 'win32') {
  checkResult(spawnSync('bash', ['build.sh'], { cwd: root, stdio: 'inherit' }), 'bash build.sh');
  process.exit(0);
}
for (const args of [['build','--release','--lib','--target','wasm32-unknown-unknown','--config','profile.release.debug=false']]) {
  checkResult(spawnSync('cargo',args,{cwd:root,stdio:'inherit'}), 'cargo');
}
mkdirSync(new URL('../dist/',import.meta.url),{recursive:true});
for(const f of ['index.html','main.js','speech.mjs','manifest.webmanifest','icon-192.png','icon-512.png','apple-touch-icon.png'])copyFileSync(new URL('../web/'+f,import.meta.url),new URL('../dist/'+f,import.meta.url));
copyFileSync(new URL('../assets/app_icon.ico',import.meta.url),new URL('../dist/favicon.ico',import.meta.url));
copyFileSync(new URL('../target/wasm32-unknown-unknown/release/yyhistory.wasm',import.meta.url),new URL('../dist/yyhistory.wasm',import.meta.url));
// 用意した画像（assets/images/。無い画像はゲームが図形で描く）
const copyImages=(from,to)=>{if(!existsSync(from))return;for(const name of readdirSync(from)){const f=path.join(from,name),t=path.join(to,name);if(statSync(f).isDirectory())copyImages(f,t);else if(/\.(png|jpg)$/i.test(name)){mkdirSync(to,{recursive:true});copyFileSync(f,t);}}};
copyImages(path.join(root,'assets/images'),path.join(root,'dist/images'));
checkResult(spawnSync(process.execPath,['tools/package_wasm.mjs','dist/yyhistory.wasm'],{cwd:root,stdio:'inherit'}), 'WASM packaging');
writeFileSync(new URL('../dist/_headers',import.meta.url),'/*.wasm\n  Content-Type: application/wasm\n  Cache-Control: no-cache\n/*.js\n  Cache-Control: no-cache\n/wasm-manifest.json\n  Cache-Control: no-cache\n/images/*\n  Cache-Control: no-cache\n/\n  Cache-Control: no-cache\n');
console.log('時空事件簿 browser build: dist/');
