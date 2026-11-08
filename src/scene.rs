//! Code-native reconstructions: all geometry is deterministic from the event ID.
//! A photo stores camera parameters and freezes animation time, so its composition persists.
use crate::gfx::Gfx;
use crate::history::{Event, Style};
type Color = [f32; 4];
fn rgb(r: f32, g: f32, b: f32) -> Color {
    [r, g, b, 1.]
}
fn tint(c: Color, n: f32) -> Color {
    [
        (c[0] + n).clamp(0., 1.),
        (c[1] + n).clamp(0., 1.),
        (c[2] + n).clamp(0., 1.),
        1.,
    ]
}
pub fn seed(e: &Event) -> u32 {
    e.id.bytes()
        .fold(123u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32))
}
fn noise(s: u32, i: u32) -> f32 {
    let mut v = s.wrapping_add(i.wrapping_mul(2654435761));
    v ^= v >> 16;
    v = v.wrapping_mul(2246822519);
    v ^= v >> 13;
    (v & 65535) as f32 / 65535.
}
/// Rasterize the same scene triangles for real photo thumbnails in the timeline.
/// Called before drawing the frame, while the geometry batch is empty.
pub fn thumbnail(g: &mut Gfx, e: &Event, heading: f32, zoom: f32, time: f32) -> Vec<u32> {
    const W: usize = 320;
    const H: usize = 180;
    debug_assert!(g.verts.is_empty());
    draw(
        g,
        e,
        [0., 0., W as f32, H as f32],
        false,
        heading,
        zoom,
        time,
    );
    let vertices = std::mem::take(&mut g.verts);
    let mut out = vec![0xffdfdfdf; W * H];
    for t in vertices.chunks_exact(3) {
        let [a, b, c] = [t[0], t[1], t[2]];
        let den = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
        if den.abs() < 0.00001 {
            continue;
        }
        let x0 = a.x.min(b.x).min(c.x).floor().max(0.) as usize;
        let x1 = a.x.max(b.x).max(c.x).ceil().clamp(0., W as f32) as usize;
        let y0 = a.y.min(b.y).min(c.y).floor().max(0.) as usize;
        let y1 = a.y.max(b.y).max(c.y).ceil().clamp(0., H as f32) as usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let u = ((b.y - c.y) * (px - c.x) + (c.x - b.x) * (py - c.y)) / den;
                let v = ((c.y - a.y) * (px - c.x) + (a.x - c.x) * (py - c.y)) / den;
                let z = 1. - u - v;
                if u >= 0. && v >= 0. && z >= 0. {
                    let mut col = [0.; 4];
                    for (k, q) in col.iter_mut().enumerate() {
                        *q = a.c[k] * u + b.c[k] * v + c.c[k] * z;
                    }
                    let old = out[y * W + x];
                    let mut bgra = [0u8; 4];
                    for (k, q) in bgra.iter_mut().take(3).enumerate() {
                        let channel = 2 - k;
                        let before = ((old >> (k * 8)) & 255) as f32;
                        *q = (col[channel] * 255. * col[3] + before * (1. - col[3])).clamp(0., 255.)
                            as u8;
                    }
                    bgra[3] = 255;
                    out[y * W + x] = u32::from_le_bytes(bgra);
                }
            }
        }
    }
    out
}
struct View<'a> {
    g: &'a mut Gfx,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
impl View<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        self.g.rect(
            self.x + x * self.w,
            self.y + y * self.h,
            self.x + (x + w) * self.w,
            self.y + (y + h) * self.h,
            c,
        )
    }
    fn tri(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32), col: Color) {
        self.g.tri(
            (self.x + a.0 * self.w, self.y + a.1 * self.h),
            (self.x + b.0 * self.w, self.y + b.1 * self.h),
            (self.x + c.0 * self.w, self.y + c.1 * self.h),
            col,
        )
    }
    fn circle(&mut self, x: f32, y: f32, r: f32, c: Color) {
        self.g
            .circle(self.x + x * self.w, self.y + y * self.h, r * self.h, 24, c)
    }
    fn line(&mut self, a: (f32, f32), b: (f32, f32), t: f32, c: Color) {
        self.g.line(
            (self.x + a.0 * self.w, self.y + a.1 * self.h),
            (self.x + b.0 * self.w, self.y + b.1 * self.h),
            t * self.h,
            c,
        )
    }
    fn person(&mut self, x: f32, y: f32, s: f32, c: Color, walk: f32) {
        self.circle(x, y - s * 0.9, s * 0.16, rgb(0.77, 0.57, 0.40));
        self.rect(x - s * 0.12, y - s * 0.7, s * 0.24, s * 0.45, c);
        self.line(
            (x - s * 0.05, y - s * 0.25),
            (x - s * 0.15 - walk, y),
            s * 0.07,
            rgb(0.23, 0.26, 0.25),
        );
        self.line(
            (x + s * 0.05, y - s * 0.25),
            (x + s * 0.15 + walk, y),
            s * 0.07,
            rgb(0.23, 0.26, 0.25),
        );
        self.line(
            (x - s * 0.1, y - s * 0.6),
            (x - s * 0.24, y - s * 0.34),
            s * 0.06,
            c,
        );
        self.line(
            (x + s * 0.1, y - s * 0.6),
            (x + s * 0.24, y - s * 0.32),
            s * 0.06,
            c,
        );
    }
    fn house(&mut self, x: f32, y: f32, w: f32, h: f32, style: Style, stone: Color, roof: Color) {
        self.tri(
            (x, y),
            (x + w * 1.5, y + w * 0.25),
            (x + w, y + w * 0.33),
            [0.17, 0.22, 0.20, 0.12],
        );
        self.rect(x, y - h, w, h, stone);
        self.tri(
            (x + w, y - h),
            (x + w + w * 0.25, y - h - w * 0.15),
            (x + w + w * 0.25, y - w * 0.15),
            tint(stone, -0.12),
        );
        self.tri(
            (x + w, y - h),
            (x + w + w * 0.25, y - w * 0.15),
            (x + w, y),
            tint(stone, -0.12),
        );
        match style {
            Style::Chinese | Style::Japanese => {
                self.tri(
                    (x - w * 0.12, y - h),
                    (x + w * 0.5, y - h - h * 0.43),
                    (x + w * 1.12, y - h),
                    roof,
                );
                self.line(
                    (x - w * 0.12, y - h),
                    (x - w * 0.18, y - h - h * 0.10),
                    0.004,
                    roof,
                );
                self.line(
                    (x + w * 1.12, y - h),
                    (x + w * 1.18, y - h - h * 0.10),
                    0.004,
                    roof,
                );
                for n in 0..4 {
                    let xx = x + w * (0.14 + n as f32 * 0.24);
                    self.rect(xx, y - h * 0.87, w * 0.045, h * 0.87, tint(roof, -0.07));
                }
            }
            Style::Desert | Style::African => {
                self.rect(x - w * 0.03, y - h - h * 0.08, w * 1.06, h * 0.1, roof);
                for n in 0..5 {
                    self.rect(
                        x + w * n as f32 * 0.23,
                        y - h - h * 0.18,
                        w * 0.09,
                        h * 0.16,
                        roof,
                    );
                }
            }
            _ => {
                self.tri(
                    (x - w * 0.07, y - h),
                    (x + w * 0.5, y - h - h * 0.48),
                    (x + w * 1.07, y - h),
                    roof,
                );
            }
        }
        self.rect(
            x + w * 0.44,
            y - h * 0.39,
            w * 0.16,
            h * 0.39,
            rgb(0.24, 0.27, 0.24),
        );
        for n in [0.17, 0.71] {
            self.rect(
                x + w * n,
                y - h * 0.72,
                w * 0.13,
                h * 0.20,
                rgb(0.29, 0.39, 0.38),
            );
        }
    }
}

pub fn draw(
    g: &mut Gfx,
    e: &Event,
    area: [f32; 4],
    aerial: bool,
    heading: f32,
    zoom: f32,
    time: f32,
) {
    let [x, y, w, h] = area;
    let scale = zoom.clamp(0.7, 2.);
    let mut v = View {
        g,
        x: x + w * (1. - scale) * 0.5,
        y: y + h * (1. - scale) * 0.5,
        w: w * scale,
        h: h * scale,
    };
    let s = seed(e);
    let desert = matches!(e.style, Style::Desert | Style::African);
    let ground = if desert {
        rgb(0.80, 0.70, 0.48)
    } else {
        rgb(0.67, 0.73, 0.55)
    };
    let roof = match e.style {
        Style::Chinese => rgb(0.39, 0.46, 0.39),
        Style::Japanese => rgb(0.25, 0.33, 0.35),
        Style::Desert | Style::African => rgb(0.76, 0.61, 0.40),
        Style::Indian => rgb(0.66, 0.40, 0.29),
        Style::Oceanic => rgb(0.85, 0.86, 0.77),
        _ => rgb(0.64, 0.36, 0.26),
    };
    let stone = if desert {
        rgb(0.88, 0.77, 0.55)
    } else {
        rgb(0.88, 0.83, 0.69)
    };
    if aerial {
        v.rect(0., 0., 1., 1., ground);
        for i in 0..25 {
            let xx = noise(s, i);
            let yy = noise(s, i + 50);
            v.rect(
                xx,
                yy,
                0.045,
                0.06,
                tint(ground, noise(s, i + 25) * 0.06 - 0.03),
            );
        }
        // A river and its lighter bank, then an irregular grid of streets.
        for i in 0..35 {
            let yy = i as f32 / 35.;
            let xx = 0.76 + (yy * 5. + noise(s, 2)).sin() * 0.07;
            v.rect(xx - 0.012, yy, 0.11, 0.034, rgb(0.78, 0.81, 0.65));
            v.rect(xx, yy, 0.085, 0.034, rgb(0.39, 0.65, 0.69));
        }
        for n in 0..7 {
            let yy = 0.08 + n as f32 * 0.143;
            v.rect(0., yy, 1., 0.018, rgb(0.84, 0.80, 0.66));
        }
        for n in 0..8 {
            let xx = 0.04 + n as f32 * 0.12;
            v.rect(xx, 0., 0.014, 1., rgb(0.84, 0.80, 0.66));
        }
        // Painter's order keeps roofs in front of the right buildings.
        for row in 0..6 {
            for col in 0..6 {
                let i = row * 6 + col;
                let xx = 0.06 + col as f32 * 0.12;
                let yy = 0.12 + row as f32 * 0.14;
                if row == 2 && (2..4).contains(&col) {
                    continue;
                }
                v.house(
                    xx,
                    yy,
                    0.047 + noise(s, i) * 0.025,
                    0.035 + noise(s, i + 91) * 0.03,
                    e.style,
                    tint(stone, noise(s, i + 44) * 0.09 - 0.045),
                    roof,
                );
                if noise(s, i + 81) > 0.3 {
                    v.circle(xx + 0.075, yy + 0.035, 0.012, rgb(0.32, 0.48, 0.32));
                }
            }
        }
        v.rect(0.27, 0.33, 0.22, 0.18, rgb(0.86, 0.80, 0.63));
        v.house(0.31, 0.44, 0.12, 0.09, e.style, stone, roof);
        for i in 0..26 {
            let xx = 0.28 + noise(s, i + 200) * 0.19;
            let yy = 0.44 + noise(s, i + 240) * 0.08;
            v.person(xx, yy, 0.014, rgb(0.40, 0.37, 0.32), 0.);
        }
        // Event-specific landmarks.
        landmark(&mut v, e, 0.54, 0.62, 0.19);
    } else {
        v.g.rect_vgrad(
            x,
            y,
            x + w,
            y + h * 0.60,
            rgb(0.58, 0.76, 0.81),
            rgb(0.91, 0.90, 0.76),
        );
        v.circle(0.78, 0.16, 0.047, rgb(0.98, 0.92, 0.68));
        for i in 0..7 {
            let xx = i as f32 * 0.18;
            v.tri(
                (xx - 0.12, 0.60),
                (xx + 0.05, 0.39 - noise(s, i) * 0.08),
                (xx + 0.24, 0.60),
                rgb(0.55, 0.63, 0.54),
            );
        }
        v.rect(0., 0.54, 1., 0.46, ground);
        v.tri((0.41, 0.54), (0.59, 0.54), (1., 1.), rgb(0.81, 0.75, 0.60));
        v.tri((0.41, 0.54), (1., 1.), (0., 1.), rgb(0.81, 0.75, 0.60));
        let offset = heading / 180. * 0.75;
        for row in 0..4 {
            let depth = row as f32 / 4.;
            let yy = 0.58 + depth * 0.34;
            let size = 0.045 + depth * 0.115;
            for side in [false, true] {
                let xx = if side {
                    0.64 + depth * 0.12 + offset
                } else {
                    0.28 - depth * 0.40 + offset
                };
                v.house(xx, yy, size, size * 0.92, e.style, stone, roof);
            }
        }
        landmark(&mut v, e, 0.50 + offset, 0.66, 0.23);
        for i in 0..21 {
            let xx = 0.31 + noise(s, i + 500) * 0.40 + offset;
            let yy = 0.65 + noise(s, i + 540) * 0.25;
            let size = 0.034 + (yy - 0.65) * 0.17;
            let walk = (time * 2. + i as f32).sin() * 0.002;
            let col = match i % 4 {
                0 => rgb(0.29, 0.42, 0.44),
                1 => rgb(0.61, 0.35, 0.26),
                2 => rgb(0.81, 0.69, 0.43),
                _ => rgb(0.42, 0.43, 0.33),
            };
            v.person(xx, yy, size, col, walk);
        }
        // Foreground vegetation and shadows give the reconstruction some depth.
        for xx in [0.045, 0.92] {
            v.rect(xx, 0.55, 0.012, 0.18, rgb(0.39, 0.33, 0.23));
            v.circle(
                xx,
                0.53,
                0.064,
                if desert {
                    rgb(0.44, 0.51, 0.28)
                } else {
                    rgb(0.31, 0.48, 0.32)
                },
            );
            v.circle(xx + 0.025, 0.49, 0.043, rgb(0.39, 0.55, 0.35));
        }
    }
}
fn landmark(v: &mut View<'_>, e: &Event, cx: f32, cy: f32, size: f32) {
    let col = rgb(0.87, 0.79, 0.61);
    let ink = rgb(0.34, 0.38, 0.33);
    match e.id {
        "giza" => {
            v.tri(
                (cx - size, cy),
                (cx, cy - size * 1.5),
                (cx + size * 0.65, cy),
                rgb(0.86, 0.74, 0.49),
            );
            v.tri(
                (cx, cy - size * 1.5),
                (cx + size, cy - size * 0.18),
                (cx + size * 0.65, cy),
                rgb(0.69, 0.57, 0.36),
            );
        }
        "apollo" => {
            v.rect(
                cx - size,
                cy - size * 0.55,
                size * 2.,
                size * 0.6,
                rgb(0.74, 0.77, 0.73),
            );
            for i in 0..6 {
                v.rect(
                    cx - size * 0.88 + i as f32 * size * 0.32,
                    cy - size * 0.43,
                    size * 0.23,
                    size * 0.19,
                    rgb(0.20, 0.38, 0.40),
                );
            }
            v.rect(
                cx - size * 0.7,
                cy + size * 0.1,
                size * 1.4,
                size * 0.05,
                ink,
            );
        }
        "wright" => {
            v.line(
                (cx - size, cy - size * 0.4),
                (cx + size, cy - size * 0.4),
                size * 0.12,
                col,
            );
            v.line(
                (cx - size, cy - size * 0.7),
                (cx + size, cy - size * 0.7),
                size * 0.12,
                col,
            );
            for k in [-0.8, 0., 0.8] {
                v.line(
                    (cx + size * k, cy - size * 0.4),
                    (cx + size * k, cy - size * 0.7),
                    size * 0.025,
                    ink,
                );
            }
            v.line((cx, cy), (cx, cy - size), size * 0.05, ink);
        }
        "railway" => {
            v.line(
                (cx - size, cy + size * 0.18),
                (cx + size, cy + size * 0.18),
                0.006,
                ink,
            );
            v.line(
                (cx - size, cy + size * 0.30),
                (cx + size, cy + size * 0.30),
                0.006,
                ink,
            );
            v.rect(
                cx - size * 0.7,
                cy - size * 0.2,
                size * 1.3,
                size * 0.35,
                rgb(0.24, 0.34, 0.29),
            );
            v.rect(
                cx - size * 0.5,
                cy - size * 0.5,
                size * 0.15,
                size * 0.3,
                ink,
            );
            for k in [-0.5, 0.4] {
                v.circle(cx + size * k, cy + size * 0.15, size * 0.12, ink);
            }
        }
        "taj" => {
            v.rect(
                cx - size * 0.6,
                cy - size * 0.5,
                size * 1.2,
                size * 0.5,
                rgb(0.94, 0.91, 0.79),
            );
            v.circle(cx, cy - size * 0.56, size * 0.32, rgb(0.94, 0.91, 0.79));
            for k in [-0.8, 0.8] {
                v.rect(
                    cx + size * k,
                    cy - size * 0.8,
                    size * 0.08,
                    size * 0.8,
                    rgb(0.94, 0.91, 0.79),
                );
            }
        }
        "sydney" => {
            for i in 0..4 {
                let xx = cx - size + i as f32 * size * 0.5;
                v.tri(
                    (xx, cy),
                    (xx + size * 0.3, cy - size * (0.8 + i as f32 * 0.1)),
                    (xx + size * 0.55, cy),
                    rgb(0.95, 0.92, 0.78),
                );
            }
        }
        "berlin" => {
            v.rect(
                cx - size,
                cy - size * 0.45,
                size * 2.,
                size * 0.45,
                rgb(0.67, 0.67, 0.60),
            );
            for i in 0..8 {
                v.rect(
                    cx - size + i as f32 * size * 0.25,
                    cy - size * 0.44,
                    size * 0.014,
                    size * 0.43,
                    rgb(0.46, 0.48, 0.45),
                );
            }
            v.rect(
                cx - size * 0.15,
                cy - size * 0.45,
                size * 0.3,
                size * 0.45,
                rgb(0.79, 0.75, 0.59),
            );
        }
        "athens" | "colosseum" => {
            v.rect(cx - size, cy - size * 0.1, size * 2., size * 0.13, col);
            for i in 0..7 {
                v.rect(
                    cx - size * 0.90 + i as f32 * size * 0.28,
                    cy - size * 0.8,
                    size * 0.12,
                    size * 0.72,
                    col,
                );
            }
            v.tri(
                (cx - size * 1.12, cy - size * 0.8),
                (cx, cy - size * 1.18),
                (cx + size * 1.12, cy - size * 0.8),
                col,
            );
        }
        _ => {
            v.house(
                cx - size * 0.5,
                cy,
                size,
                size * 0.6,
                e.style,
                col,
                if matches!(e.style, Style::Chinese | Style::Japanese) {
                    rgb(0.29, 0.39, 0.35)
                } else {
                    rgb(0.64, 0.37, 0.27)
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frozen_photo_has_deterministic_pixels() {
        let mut g = Gfx::new(1600, 900);
        let e = &crate::history::EVENTS[4];
        let a = thumbnail(&mut g, e, 0., 1., 12.);
        let b = thumbnail(&mut g, e, 0., 1., 12.);
        assert_eq!(a, b);
        assert_eq!(a.len(), 320 * 180);
        assert!(a.windows(2).filter(|v| v[0] != v[1]).count() > 1000);
    }
    #[test]
    #[ignore]
    fn render_postcards() {
        std::fs::create_dir_all("output/review").unwrap();
        let mut g = Gfx::new(1600, 900);
        for id in ["qin", "revolution", "giza", "taj", "wright", "sydney"] {
            let e = crate::history::EVENTS.iter().find(|e| e.id == id).unwrap();
            let pixels = thumbnail(&mut g, e, 0., 1., 12.);
            let rgba: Vec<u8> = pixels
                .iter()
                .flat_map(|p| {
                    let [b, g, r, a] = p.to_le_bytes();
                    [r, g, b, a]
                })
                .collect();
            image::save_buffer(
                format!("output/review/{id}.png"),
                &rgba,
                320,
                180,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
}
