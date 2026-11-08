"""Code-native compass / timeline app icon. Requires Pillow."""
from pathlib import Path
from PIL import Image, ImageDraw
root=Path(__file__).resolve().parents[1]
image=Image.new('RGBA',(512,512),(248,246,236));d=ImageDraw.Draw(image)
d.rounded_rectangle((24,24,488,488),radius=104,fill=(44,86,72))
d.ellipse((102,78,410,386),outline=(225,224,196),width=8)
d.line((256,112,256,352),fill=(225,224,196),width=3)
d.line((136,232,376,232),fill=(225,224,196),width=3)
d.polygon([(256,116),(213,275),(256,253)],fill=(235,188,116))
d.polygon([(256,116),(299,275),(256,253)],fill=(249,241,215))
d.ellipse((242,218,270,246),fill=(44,86,72))
d.line((130,422,382,422),fill=(225,224,196),width=6)
for x in [156,256,356]:d.ellipse((x-10,412,x+10,432),fill=(235,188,116))
for size,name in [(192,'icon-192.png'),(512,'icon-512.png'),(180,'apple-touch-icon.png')]:
    image.resize((size,size),Image.Resampling.LANCZOS).save(root/'web'/name)
image.save(root/'assets/app_icon.ico',sizes=[(16,16),(32,32),(48,48),(64,64),(128,128),(256,256)])
