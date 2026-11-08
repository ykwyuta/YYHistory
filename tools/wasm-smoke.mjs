// Exercise the real production Wasm ABI, without a browser or graphics driver.
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import assert from 'node:assert/strict';
const mod=new WebAssembly.Module(readFileSync(new URL('../dist/yyhistory.wasm',import.meta.url)));
let uiScale=1;let wasm;let labels=[];let saved=null;let nextTexture=1;let bitmaps=0;let frame=[];let transform=[1,1,0,0];const textures={};
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
 js_bmp_create:()=>++bitmaps,
 js_save:(slot,p,n)=>{saved=Uint8Array.from(new Uint8Array(wasm.memory.buffer,p,n));},
 js_log:(p,n,level)=>{if(level===2)throw Error(decode(p,n));},
});
wasm=new WebAssembly.Instance(mod,{env}).exports;
const tick=()=>wasm.yy_frame(0.016,1);
const key=(k)=>{wasm.yy_event(4,k,1,0);tick();wasm.yy_event(4,k,0,0);};
const has=(word)=>assert.ok(labels.some(t=>t.includes(word)),`Expected text: ${word}`);
wasm.yy_init(1360,720,0);tick();has('YYHistory');has('上空へ');
assert.ok(textures[1].w>=720);
mkdirSync(new URL('../output/review/',import.meta.url),{recursive:true});
writeFileSync(new URL('../output/review/world-frame.json',import.meta.url),JSON.stringify({width:Math.round(1360/uiScale),height:Math.round(720/uiScale),textures,frame}));
key(13);has('現場に降りる');key(13);has('撮影する');key(32);assert.ok(saved?.length>20);assert.ok(bitmaps>0);has('1 / 32');
key(0x71);has('旅の記憶');has('秦の中国統一');
const before=Buffer.from(saved);const ptr=wasm.yy_alloc(saved.length);new Uint8Array(wasm.memory.buffer,ptr,saved.length).set(saved);wasm.yy_loaded(ptr,saved.length);tick();has('写真帳を読み込みました');has('1 / 32');
// Imported photos survive a full return through the exported loader.
assert.equal(Buffer.from(saved).toString('hex'),before.toString('hex'));
wasm.yy_event(6,900,420,0);wasm.yy_event(8,1,0,0);tick();has('YYHistory');
const query=new TextEncoder().encode('秦');const qp=wasm.yy_alloc(query.length);new Uint8Array(wasm.memory.buffer,qp,query.length).set(query);wasm.yy_search(qp,query.length);tick();has('地域別年表  /  1件');
console.log('Production Wasm smoke passed: map → aerial → street → photo → album → import → compact.');
