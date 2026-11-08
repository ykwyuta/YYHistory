"""Render the recorded production-Wasm drawing commands to an offline QA image.
This is a software render of the drawing API, not a browser screenshot.
"""
import json, base64, math, textwrap
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
root=Path(__file__).resolve().parents[1]
data=json.loads((root/'output/review/world-frame.json').read_text(encoding='utf-8'))
image=Image.new('RGBA',(data['width'],data['height']),(249,247,240,255))
textures={int(k):Image.frombytes('RGBA',(v['w'],v['h']),base64.b64decode(v['rgba'])) for k,v in data['textures'].items()}
def color(c): return tuple(round(max(0,min(1,v))*255) for v in c)
font_path='C:/Windows/Fonts/YuGothR.ttc'
for cmd in data['frame']:
    kind=cmd['kind']
    if kind=='mesh':
        verts=cmd['vertices'];tex=cmd['tex'];t=cmd['transform']
        def xy(x,y): return ((x*t[0]+t[2]+1)/2*image.width,(1-y*t[1]-t[3])/2*image.height)
        if tex:
            pts=[xy(verts[i],verts[i+1]) for i in range(0,len(verts),8)]
            x0=round(min(p[0] for p in pts));y0=round(min(p[1] for p in pts));x1=round(max(p[0] for p in pts));y1=round(max(p[1] for p in pts))
            if x1>x0 and y1>y0:image.alpha_composite(textures[tex].resize((x1-x0,y1-y0)),(x0,y0))
        else:
            overlay=Image.new('RGBA',image.size);d=ImageDraw.Draw(overlay)
            for i in range(0,len(verts),24):
                tri=verts[i:i+24];d.polygon([xy(tri[j],tri[j+1]) for j in (0,8,16)],fill=color(tri[4:8]))
            image=Image.alpha_composite(image,overlay)
        continue
    d=ImageDraw.Draw(image);a=cmd.get('args',[])
    if kind in ('rect','round','stroke','strokeRound'):
        box=tuple(a[:4]);
        if kind=='rect':d.rectangle(box,fill=color(a[4:8]))
        if kind=='round':d.rounded_rectangle(box,radius=a[4],fill=color(a[5:9]))
        if kind=='stroke':d.rectangle(box,outline=color(a[5:9]),width=max(1,round(a[4])))
        if kind=='strokeRound':d.rounded_rectangle(box,radius=a[4],outline=color(a[6:10]),width=max(1,round(a[5])))
    elif kind=='circle':
        x,y,r=a[:3];d.ellipse((x-r,y-r,x+r,y+r),fill=color(a[3:7]))
    elif kind=='text':
        x,y,w,h,f=a[:5];size=[12,14,14,18,20,40,30][int(f)];font=ImageFont.truetype(font_path,size)
        # Mirror the bridge's wrapping and vertical centering for layout review.
        lines=[]
        for paragraph in cmd['text'].split('\n'):
            line=''
            for c in paragraph:
                if line and d.textlength(line+c,font=font)>w:lines.append(line);line=''
                line+=c
            lines.append(line)
        yy=y+(h-len(lines)*size*1.33)/2 if int(f) in [2,4,5,6] else y
        for line in lines:
            xx=x+(w-d.textlength(line,font=font))/2 if int(f) in [2,4,5,6] else x
            d.text((xx,yy-2),line,font=font,fill=color(a[5:9]),stroke_width=0);yy+=size*1.33
image.convert('RGB').save(root/'output/review/world-layout.png')
print(root/'output/review/world-layout.png')
