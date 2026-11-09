//! 時空転移の演出：渦を巻く時空のトンネルの中を、針の回る時計がいくつも飛んでくる。
use super::*;

/// 転移の演出の長さ（秒）
pub const SHIFT_SECS: f32 = 3.0;

/// 回転した楕円（三角形の扇で塗る）
fn ellipse(g: &mut Gfx, c: (f32, f32), rx: f32, ry: f32, rot: f32, col: [f32; 4]) {
    let seg = 28;
    let (s, co) = rot.sin_cos();
    let pt = |a: f32| {
        let (x, y) = (a.cos() * rx, a.sin() * ry);
        (c.0 + x * co - y * s, c.1 + x * s + y * co)
    };
    for k in 0..seg {
        let a0 = k as f32 / seg as f32 * std::f32::consts::TAU;
        let a1 = (k + 1) as f32 / seg as f32 * std::f32::consts::TAU;
        g.tri(c, pt(a0), pt(a1), col);
    }
}

/// 楕円の上の点（角度 a、半径の割合 f）
fn on(c: (f32, f32), rx: f32, ry: f32, rot: f32, a: f32, f: f32) -> (f32, f32) {
    let (s, co) = rot.sin_cos();
    let (x, y) = (a.cos() * rx * f, a.sin() * ry * f);
    (c.0 + x * co - y * s, c.1 + x * s + y * co)
}

fn hue(h: f32, a: f32) -> [f32; 4] {
    let h = h.rem_euclid(1.) * 6.;
    let x = 1. - ((h % 2.) - 1.).abs();
    let (r, g, b) = match h as i32 {
        0 => (1., x, 0.),
        1 => (x, 1., 0.),
        2 => (0., 1., x),
        3 => (0., x, 1.),
        4 => (x, 0., 1.),
        _ => (1., 0., x),
    };
    // 少し白を混ぜて、やわらかい色に
    [0.35 + r * 0.6, 0.35 + g * 0.6, 0.45 + b * 0.55, a]
}

/// ひとつの時計（ゆがみながら回る）
fn clock(g: &mut Gfx, c: (f32, f32), size: f32, seed: f32, t: f32, a: f32) {
    let wob = (t * 2.6 + seed * 7.).sin();
    let rx = size * (1. + 0.22 * wob);
    let ry = size * (1. - 0.18 * wob);
    let rot = t * (0.6 + seed) * if seed > 0.5 { 1. } else { -1. } + seed * 6.;
    let rims = [
        [0.90, 0.75, 0.42, a],
        [0.95, 0.55, 0.62, a],
        [0.45, 0.80, 0.85, a],
        [0.70, 0.62, 0.95, a],
    ];
    let rim = rims[(seed * 97.) as usize % rims.len()];
    ellipse(
        g,
        (c.0 + size * 0.08, c.1 + size * 0.12),
        rx * 1.12,
        ry * 1.12,
        rot,
        [0., 0., 0.08, 0.25 * a],
    );
    ellipse(g, c, rx * 1.12, ry * 1.12, rot, rim);
    ellipse(g, c, rx, ry, rot, [0.99, 0.96, 0.86, a]);
    // 目盛り
    for k in 0..12 {
        let p = on(c, rx, ry, rot, k as f32 / 12. * std::f32::consts::TAU, 0.82);
        let r = if k % 3 == 0 { size * 0.09 } else { size * 0.05 };
        g.circle(p.0, p.1, r.max(0.8), 8, [0.25, 0.20, 0.30, a]);
    }
    // 針：早回し（逆回りの時計もある）
    let dir = if seed > 0.35 { 1. } else { -1. };
    let m = t * (3. + seed * 6.) * dir + seed * 20.;
    let hh = m / 12.;
    let w = (size * 0.07).max(1.);
    g.line(
        c,
        on(c, rx, ry, rot, hh, 0.45),
        w * 1.4,
        [0.20, 0.16, 0.26, a],
    );
    g.line(c, on(c, rx, ry, rot, m, 0.7), w, [0.20, 0.16, 0.26, a]);
    g.circle(c.0, c.1, w * 1.6, 10, [0.75, 0.30, 0.25, a]);
}

impl Game {
    /// 転移の演出（三角形の層）。文字は shift_ui が重ねる
    pub(super) fn shift_geometry(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let t = self.shift_t;
        let k = (t / SHIFT_SECS).clamp(0., 1.);
        let a = (k / 0.12).min(1.);
        let (cx, cy) = (w / 2., h * 0.45);
        let big = w.max(h);
        let g = &mut self.gfx;
        g.rect_vgrad(0., 0., w, h, [0.16, 0.06, 0.30, a], [0.04, 0.10, 0.28, a]);
        // 時空の渦
        for arm in 0..7 {
            let base = arm as f32 / 7. * std::f32::consts::TAU + t * 1.3;
            let col = hue(arm as f32 / 7. + t * 0.08, 0.32 * a);
            let mut prev = None;
            for s in 0..48 {
                let f = s as f32 / 47.;
                let r = 8. + f * f * big * 0.75;
                let ang = base + f * 5.5;
                let p = (cx + ang.cos() * r, cy + ang.sin() * r * 0.82);
                if let Some(q) = prev {
                    g.line(q, p, 2. + f * 46., col);
                }
                prev = Some(p);
            }
        }
        g.radial(
            cx,
            cy,
            big * 0.22,
            40,
            [1., 1., 1., 0.55 * a],
            [0.6, 0.85, 1., 0.],
        );
        // 飛んでくる時計（遠いものから描く）
        let mut clocks: Vec<(f32, f32, f32)> = (0..30)
            .map(|n| {
                let seed = ((n as u32).wrapping_mul(2654435761) % 1000) as f32 / 1000.;
                let p = (seed * 3.1 + t * (0.32 + seed * 0.18)).fract();
                (p, seed, n as f32)
            })
            .collect();
        clocks.sort_by(|x, y| x.0.total_cmp(&y.0));
        for (p, seed, n) in clocks {
            let ang = n * 2.399 + t * 0.9 + p * 1.8;
            let dist = p.powf(1.6) * big * 0.62;
            let size = 6. + p * p * big * 0.075;
            let fade = (p * 5.).min(1.) * ((1. - p) * 6.).min(1.) * a;
            clock(
                g,
                (cx + ang.cos() * dist, cy + ang.sin() * dist * 0.8),
                size,
                seed,
                t,
                fade,
            );
        }
    }
}
