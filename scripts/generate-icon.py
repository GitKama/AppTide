"""Regenerate AppTide's authored A/wave icon. Requires Pillow for PNG/ICO export."""
from pathlib import Path
from math import hypot
from PIL import Image, ImageDraw

root = Path(__file__).resolve().parent.parent / 'assets'
root.mkdir(exist_ok=True)
(root / 'apptide.svg').write_text('''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" rx="54" fill="#076d72"/><path d="M57 188 L108 62 H148 L199 188" fill="none" stroke="white" stroke-width="18" stroke-linecap="round" stroke-linejoin="round"/><path d="M62 156 C95 127 122 184 158 155 C173 144 184 145 195 153" fill="none" stroke="#b2e9de" stroke-width="16" stroke-linecap="round"/></svg>''', encoding='utf-8')
im = Image.new('RGBA', (1024, 1024))
d = ImageDraw.Draw(im)
d.rounded_rectangle((0, 0, 1023, 1023), radius=216, fill='#076d72')
pts = [(57, 188), (108, 62), (148, 62), (199, 188)]
d.line([(x*4, y*4) for x, y in pts], fill='white', width=72, joint='curve')
for x, y in [pts[0], pts[-1]]:
    d.ellipse(((x-9)*4, (y-9)*4, (x+9)*4, (y+9)*4), fill='white')
points = []
for c in [((62,156),(95,127),(122,184),(158,155)), ((158,155),(173,144),(184,145),(195,153))]:
    for i in range(61):
        t = i/60
        u = 1-t
        points.append(tuple(4*(u**3*c[0][j]+3*u*u*t*c[1][j]+3*u*t*t*c[2][j]+t**3*c[3][j]) for j in (0,1)))
left, right = [], []
for i, (x, y) in enumerate(points):
    before, after = points[max(0, i-1)], points[min(len(points)-1, i+1)]
    dx, dy = after[0]-before[0], after[1]-before[1]
    length = hypot(dx, dy)
    nx, ny = -dy/length*32, dx/length*32
    left.append((x+nx, y+ny))
    right.append((x-nx, y-ny))
d.polygon(left + list(reversed(right)), fill='#b2e9de')
for x, y in [(62,156),(195,153)]:
    d.ellipse(((x-8)*4,(y-8)*4,(x+8)*4,(y+8)*4), fill='#b2e9de')
im = im.resize((256,256), Image.Resampling.LANCZOS)
im.save(root/'apptide.png')
im.save(root/'apptide.ico', sizes=[(s,s) for s in [16,24,32,48,64,128,256]])
