// Exercise the real production Wasm ABI, without a browser or graphics driver.
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import assert from 'node:assert/strict';
const mod=new WebAssembly.Module(readFileSync(new URL('../dist/yyhistory.wasm',import.meta.url)));
let uiScale=1;let wasm;let labels=[];let saved=null;let nextTexture=1;let frame=[];let transform=[1,1,0,0];const textures={};
const decode=(p,n)=>new TextDecoder().decode(new Uint8Array(wasm.memory.buffer,p,n));
const env=Object.fromEntries(WebAssembly.Module.imports(mod).map(({name})=>[name,()=>0]));
Object.assign(env,{
 js_2d_begin:(ui)=>{uiScale=ui;},
 js_random:()=>12345,js_lang_get:()=>0,js_voice_get:()=>0,
 js_tex_create:(w,h,p)=>{const id=nextTexture++;textures[id]={w,h,rgba:Buffer.from(new Uint8Array(wasm.memory.buffer,p,w*h*4)).toString('base64')};return id;},
 js_gl_begin:()=>{labels=[];frame=[];},
 js_gl_xform:(...t)=>{transform=t;},
 js_gl_draw:(p,n,tex)=>{frame.push({kind:'mesh',tex,transform:[...transform],vertices:Array.from(new Float32Array(wasm.memory.buffer,p,n*8))});},
 js_fill_rect:(...args)=>frame.push({kind:'rect',args}),js_fill_round:(...args)=>frame.push({kind:'round',args}),
 js_stroke_rect:(...args)=>frame.push({kind:'stroke',args}),js_stroke_round:(...args)=>frame.push({kind:'strokeRound',args}),
 js_fill_circle:(...args)=>frame.push({kind:'circle',args}),
 js_text:(p,n,x,y,w,h,font,...color)=>{const text=decode(p,n);labels.push(text);frame.push({kind:'text',text,args:[x,y,w,h,font,...color]});},
 js_measure:(p,n)=>decode(p,n).length*14,js_measure_h:()=>24,
 js_save:(slot,p,n)=>{saved=Uint8Array.from(new Uint8Array(wasm.memory.buffer,p,n));},
 js_log:(p,n,level)=>{if(level===2)throw Error(decode(p,n));},
});
wasm=new WebAssembly.Instance(mod,{env}).exports;
const tick=(n=1)=>{for(let i=0;i<n;i++)wasm.yy_frame(0.05,1);};
const key=(k)=>{wasm.yy_event(4,k,1,0);tick();wasm.yy_event(4,k,0,0);};
const has=(word)=>assert.ok(labels.some(t=>t.includes(word)),`Expected text: ${word}`);
const click=(word)=>{const t=frame.find(f=>f.kind==='text'&&f.text.includes(word));assert.ok(t,`No text to click: ${word}`);const [x,y,w,h]=t.args;const px=(x+Math.max(w,4)/2)*uiScale,py=(y+Math.max(h,4)/2)*uiScale;wasm.yy_event(0,px,py,0);wasm.yy_event(1,0,px,py);wasm.yy_event(2,0,px,py);tick();};
wasm.yy_init(1360,720,0);tick();has('YYHistory');has('転移先の時代');has('共和政ローマ');has('転移する');
assert.ok(textures[1].w>=720);
mkdirSync(new URL('../output/review/',import.meta.url),{recursive:true});
writeFileSync(new URL('../output/review/world-frame.json',import.meta.url),JSON.stringify({width:Math.round(1360/uiScale),height:Math.round(720/uiScale),textures,frame}));
// 転移してローマのフォルムに着く
key(13);has('時 空 転 移');tick(40);has('フォルム・ロマヌム');has('この場所にいる人物');
// 人物に会う → 人物一覧表に登録
click('マルクス・アントニウス');has('わたしはマルクス・アントニウス');has('1 / 3');
key(13);has('2 / 3');key(13);key(13);has('人物 1/43');
// 出来事を観察する
click('ルペルカリア祭の冠');has('観察');has('史料：');key(27);has('出来事 1/45');
tick(20);assert.ok(saved?.length>20,'autosave');
// 図鑑
key(0x71);has('人物一覧表');has('No.001');has('マルクス・アントニウス');has('？？？');
click('年表');has('紀元前44年2月15日');has('ルペルカリア祭の冠');
click('出来事集');has('ルペルカリア祭の冠');
// 記録の読み込み
const before=Buffer.from(saved);const ptr=wasm.yy_alloc(saved.length);new Uint8Array(wasm.memory.buffer,ptr,saved.length).set(saved);wasm.yy_loaded(ptr,saved.length);tick();has('旅の記録を読み込みました');has('人物 1/43');
tick(20);assert.equal(Buffer.from(saved).toString('hex'),before.toString('hex'));
// 検索欄の位置を返す
assert.ok(wasm.yy_search_box(2)>100);
// ローマの記録がそろった状態を読み込み、アカシックレコードの試練に挑む
{const enc=new TextEncoder();const b=[...enc.encode('YYHIST'),3,0,0,2,1,0];
 const list=ids=>{b.push(ids.length,0);for(const id of ids){const e=enc.encode(id);b.push(e.length,...e);}};
 list(['caesar','calpurnia','brutus','cassius','antony','cicero']);list(['lupercalia','ides','capitol_retreat','tellus_senate','funeral']);
 let h=2166136261;for(const v of b){h=Math.imul(h^v,16777619)>>>0;}b.push(h&255,(h>>8)&255,(h>>16)&255,h>>>24);
 const p=wasm.yy_alloc(b.length);new Uint8Array(wasm.memory.buffer,p,b.length).set(b);wasm.yy_loaded(p,b.length);tick();}
key(0x1b);key(13);tick(40);has('アカシックレコードの試練に挑む');click('アカシックレコードの試練に挑む');has('試練を始める');
key(13);has('第1問 / 5');
for(const k of [2,3,2,1,4]){key(48+k);has('正解！');key(13);}
has('5問中 5問 正解');has('デナリウス銀貨');has('秘宝 1/7');
wasm.yy_event(6,900,420,0);wasm.yy_event(8,1,0,0);tick();has('YYHistory');
console.log('Production Wasm smoke passed: atlas → shift → field → meet → observe → codex → import → trial → treasure → compact.');
