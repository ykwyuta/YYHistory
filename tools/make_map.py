"""Build the offline atlas from public-domain Natural Earth 1:110m land.
Source: https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_110m_land.geojson
No contemporary country borders are used as historical boundaries.
"""
import json, math, random
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter
root = Path(__file__).resolve().parents[1]
w, h = 1440, 720
mask = Image.new('L', (w, h)); d = ImageDraw.Draw(mask)
for f in json.loads((root/'assets/land.geojson').read_text())['features']:
    geom = f['geometry']
    polygons = geom['coordinates'] if geom['type']=='MultiPolygon' else [geom['coordinates']]
    for polygon in polygons:
        for i, ring in enumerate(polygon):
            pts = [((x+180)/360*w, (90-y)/180*h) for x,y,*_ in ring]
            d.polygon(pts, fill=255 if i==0 else 0)
shore = mask.filter(ImageFilter.GaussianBlur(5))
rng = random.Random(16)
image = Image.new('RGB',(w,h)); pixels=image.load(); land=mask.load(); coast=shore.load()
for y in range(h):
    lat=90-y/h*180
    for x in range(w):
        lon=x/w*360-180
        n = math.sin(x*.08+math.sin(y*.041)*4)*math.sin(y*.12)+rng.uniform(-1,1)
        if land[x,y]:
            desert = (abs(lat-24)<12 and -19<lon<60) or (32<lat<47 and 48<lon<107) or (-32<lat<-18 and 116<lon<144)
            ice=abs(lat)>68
            base=(219,203,159) if desert else (156,184,146) if abs(lat)<26 else (182,199,164)
            if ice: base=(226,234,220)
            v=n*3 + (coast[x,y]/255-1)*12
            pixels[x,y]=tuple(max(0,min(255,int(c+v))) for c in base)
        else:
            v=coast[x,y]/255*19+n
            pixels[x,y]=(int(165+v),int(204+v),int(214+v))
image.save(root/'assets/world.png',optimize=True)
print('Offline map:',root/'assets/world.png')
