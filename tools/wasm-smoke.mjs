// Exercise the real production Wasm ABI, without a browser or graphics driver.
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import assert from 'node:assert/strict';
const mod=new WebAssembly.Module(readFileSync(new URL('../dist/yyhistory.wasm',import.meta.url)));
let uiScale=1;let wasm;let labels=[];let saved=null;let nextTexture=1;let frame=[];let transform=[1,1,0,0];const textures={};
const decode=(p,n)=>new TextDecoder().decode(new Uint8Array(wasm.memory.buffer,p,n));
const fontSizes=[12,14,14,18,20,40,30];
// Approximate full-width Japanese glyphs and proportional ASCII in the layout regression.
const textWidth=(s,f)=>[...s].reduce((w,c)=>w+fontSizes[f]*(c.codePointAt(0)<128?0.6:1),0);
let atlasLayoutCheck=false;
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
 js_measure:(p,n,f)=>atlasLayoutCheck?textWidth(decode(p,n),f):decode(p,n).length*14,
 js_measure_h:(p,n,f,w)=>atlasLayoutCheck?Math.max(1,Math.ceil(textWidth(decode(p,n),f)/w))*fontSizes[f]*1.33:24,
 js_img_load:(p,n)=>decode(p,n).endsWith('/eras/tokyo.jpg')?1:0,
 js_img_ready:id=>id===1,
 js_img_draw:(id,...args)=>frame.push({kind:'image',id,args}),
 js_save:(slot,p,n)=>{saved=Uint8Array.from(new Uint8Array(wasm.memory.buffer,p,n));},
 js_log:(p,n,level)=>{if(level===2)throw Error(decode(p,n));},
});
wasm=new WebAssembly.Instance(mod,{env}).exports;
const tick=(n=1)=>{for(let i=0;i<n;i++)wasm.yy_frame(0.05,1);};
const key=(k)=>{wasm.yy_event(4,k,1,0);tick();wasm.yy_event(4,k,0,0);};
const has=(word)=>assert.ok(labels.some(t=>t.includes(word)),`Expected text: ${word}`);
const click=(word)=>{const t=frame.find(f=>f.kind==='text'&&f.text.includes(word));assert.ok(t,`No text to click: ${word}`);const [x,y,w,h]=t.args;const px=(x+Math.max(w,4)/2)*uiScale,py=(y+Math.max(h,4)/2)*uiScale;wasm.yy_event(0,px,py,0);wasm.yy_event(1,0,px,py);wasm.yy_event(2,0,px,py);tick();};
wasm.yy_init(1360,720,0);tick();has('時空事件簿');has('転移先の時代');has('共和政ローマ');has('転移する');
// はじめて遊ぶとき：時空探偵のおはなしから始まる
const checkNoTextOverlap=()=>{
 let start=0;
 for(let i=0;i<frame.length;i++)if(frame[i].kind==='rect'&&frame[i].args[0]===0&&frame[i].args[1]===0&&frame[i].args[2]>=900)start=i;
 const texts=frame.slice(start).filter(f=>f.kind==='text').map(f=>{
  const [x,y,w,h]=f.args;
  const tw=Math.min(w,f.text.length*14);
  const cx=x+w/2;
  return {text:f.text,x0:cx-tw/2,x1:cx+tw/2,y0:y,y1:y+h};
 });
 for(let i=0;i<texts.length;i++)for(let j=i+1;j<texts.length;j++){
  const a=texts[i],b=texts[j];
  const ox=Math.min(a.x1,b.x1)-Math.max(a.x0,b.x0);
  const oy=Math.min(a.y1,b.y1)-Math.max(a.y0,b.y0);
  assert.ok(ox<=2||oy<=2,`Overlapping text: "${a.text}" and "${b.text}"`);
 }
};
tick(12);has('じくう じけんぼ');has('はじめる');key(13);has('ようこそ、時空探偵！');checkNoTextOverlap();key(13);has('タイムマシンで出発！');checkNoTextOverlap();
for(let i=0;i<5;i++){key(13);checkNoTextOverlap();}has('ぼくはナビのクロノ！');key(13);has('出発！');checkNoTextOverlap();key(13);
has('行きたい時代を選んで');
assert.ok(textures[1].w>=720);
mkdirSync(new URL('../output/review/',import.meta.url),{recursive:true});
writeFileSync(new URL('../output/review/world-frame.json',import.meta.url),JSON.stringify({width:Math.round(1360/uiScale),height:Math.round(720/uiScale),textures,frame}));
// 転移してローマのフォルムに着く
key(13);has('これから行く時代について、説明するね！');has('どんな場所？');key(13);has('それまでに何があった？');key(27);has('時 空 転 移');tick(70);has('フォルム・ロマヌム');has('この場所にいる人物');
// 人物に会う → 人物一覧表に登録
click('マルクス・アントニウス');has('きのうまでの友が');has('きいてみよう');
click('これからどうするの？');has('あなた');has('血のついた衣');
key(13);has('早く帰りなさい');has('さようなら');key(13);has('時空の復元力だ');key(13);has('人物 1/129');
// 出来事を観察する
click('ルペルカリア祭の冠');has('観察');has('史料：');key(27);has('出来事 1/180');
tick(20);assert.ok(saved?.length>20,'autosave');
// 図鑑
key(0x71);has('人物一覧表');has('No.001');has('マルクス・アントニウス');has('？？？');
click('年表');has('紀元前44年2月15日');has('ルペルカリア祭の冠');
click('出来事集');has('ルペルカリア祭の冠');
click('なぜ？ノート');has('クロノに聞いたなぜ？');
// 記録の読み込み
const before=Buffer.from(saved);const ptr=wasm.yy_alloc(saved.length);new Uint8Array(wasm.memory.buffer,ptr,saved.length).set(saved);wasm.yy_loaded(ptr,saved.length);tick();has('旅の記録を読み込みました');has('人物 1/129');
tick(20);assert.equal(Buffer.from(saved).toString('hex'),before.toString('hex'));
// 検索欄の位置を返す
assert.ok(wasm.yy_search_box(2)>100);
// ローマの記録がそろった状態を読み込み、アカシックレコードの試練に挑む
{const enc=new TextEncoder();const b=[...enc.encode('YYHIST'),3,0,0,2,1,0];
 const list=ids=>{b.push(ids.length,0);for(const id of ids){const e=enc.encode(id);b.push(e.length,...e);}};
 list(['caesar','calpurnia','brutus','cassius','antony','cicero']);list(['lupercalia','ides','capitol_retreat','tellus_senate','funeral']);
 let h=2166136261;for(const v of b){h=Math.imul(h^v,16777619)>>>0;}b.push(h&255,(h>>8)&255,(h>>16)&255,h>>>24);
 const p=wasm.yy_alloc(b.length);new Uint8Array(wasm.memory.buffer,p,b.length).set(b);wasm.yy_loaded(p,b.length);tick();}
key(0x1b);key(13);tick(70);has('アカシックレコードの試練に挑む');click('アカシックレコードの試練に挑む');has('試練を始める');
key(13);has('第1問 / 5');
for(const k of [2,3,2,1,4]){key(48+k);has('正解！');key(13);}
has('5問中 5問 正解');has('デナリウス銀貨');has('秘宝 1/37');
// Resume Tokyo with a return button, global counters and a loaded era image.
{const b=[...new TextEncoder().encode('YYHIST\x07\0\x05tokyo'),0,255,1,0,5,...new TextEncoder().encode('tokyo'),0,0,0,0,0,0,0,0];
 let h=2166136261;for(const v of b){h=Math.imul(h^v,16777619)>>>0;}b.push(h&255,(h>>8)&255,(h>>16)&255,h>>>24);
 const p=wasm.yy_alloc(b.length);new Uint8Array(wasm.memory.buffer,p,b.length).set(b);wasm.yy_loaded(p,b.length);}
atlasLayoutCheck=true;
const textBox=(label)=>{
 const command=frame.find(f=>f.kind==='text'&&f.text===label);
 assert.ok(command,`Expected text: ${label}`);
 return command.args.slice(0,4);
};
const disjoint=(a,b)=>a[0]+a[2]<=b[0]||b[0]+b[2]<=a[0]||a[1]+a[3]<=b[1]||b[1]+b[3]<=a[1];
for(const [w,h,compact] of [[900,420,1],[1000,480,1],[1360,720,0]]){
 wasm.yy_event(6,w,h,0);wasm.yy_event(8,compact,0,0);tick(2);
 const back=textBox('明治の東京へ戻る');
 const global=textBox('人物 0/129　出来事 0/180　秘宝 0/37');
 assert.ok(disjoint(back,global),`${w}: return button overlaps global progress`);
 assert.ok(disjoint(global,textBox('使い方')),`${w}: progress overlaps help`);
 const date=textBox('1900年　／　東京（日本）');
 const count=textBox('人物 0/3　出来事 0/3');
 const title=textBox('明治の東京 ― 津田梅子と女子英学塾');
 const actions=['転移する','🔊 きく','クロノの説明'].map(textBox);
 const boxes=[date,count,title,...actions];
 for(let i=0;i<boxes.length;i++)for(let j=i+1;j<boxes.length;j++){
  assert.ok(disjoint(boxes[i],boxes[j]),`${w}: overlapping era card text: ${i}, ${j}`);
 }
 assert.ok(textWidth('1900年　／　東京（日本）',0)<=date[2],`${w}: date wraps out of its row`);
 const art=frame.find(f=>f.kind==='image'&&f.id===1);
 if(w>=1000)assert.ok(art,`${w}: missing era image`);
 if(art)assert.ok(art.args[2]<title[0],`${w}: era image overlaps card text`);
 for(const f of frame.filter(f=>f.kind==='text'&&f.args[4]===2&&f.args[3]===22&&f.args[1]<date[1])){
  assert.ok(textWidth(f.text,2)<=f.args[2],`${w}: map label wraps: ${f.text}`);
 }
}
click('クロノの説明');has('どんな場所？');
console.log('Production Wasm smoke passed: story, atlas, briefing, field, codex, saves, trial, and atlas layout at three widths.');
