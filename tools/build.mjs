// Cross-platform browser build; no npm dependencies or Bash required.
import { spawnSync } from 'node:child_process';
import { mkdirSync, copyFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const root=fileURLToPath(new URL('../',import.meta.url));
for (const args of [['build','--release','--lib','--target','wasm32-unknown-unknown','--config','profile.release.debug=false']]) {
  const r=spawnSync('cargo',args,{cwd:root,stdio:'inherit'});if(r.status!==0) process.exit(r.status||1);
}
mkdirSync(new URL('../dist/',import.meta.url),{recursive:true});
for(const f of ['index.html','main.js','speech.mjs','manifest.webmanifest','icon-192.png','icon-512.png','apple-touch-icon.png'])copyFileSync(new URL('../web/'+f,import.meta.url),new URL('../dist/'+f,import.meta.url));
copyFileSync(new URL('../assets/app_icon.ico',import.meta.url),new URL('../dist/favicon.ico',import.meta.url));
copyFileSync(new URL('../target/wasm32-unknown-unknown/release/yyhistory.wasm',import.meta.url),new URL('../dist/yyhistory.wasm',import.meta.url));
const r=spawnSync(process.execPath,['tools/package_wasm.mjs','dist/yyhistory.wasm'],{cwd:root,stdio:'inherit'});if(r.status!==0)process.exit(r.status||1);
writeFileSync(new URL('../dist/_headers',import.meta.url),'/*.wasm\n  Content-Type: application/wasm\n  Cache-Control: no-cache\n/*.js\n  Cache-Control: no-cache\n/wasm-manifest.json\n  Cache-Control: no-cache\n/\n  Cache-Control: no-cache\n');
console.log('YYHistory browser build: dist/');
