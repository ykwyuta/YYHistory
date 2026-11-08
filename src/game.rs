use crate::{
    gfx::{Bitmap, Font, Gfx, Texture},
    history::*,
    platform, scene, Ev,
};
const INK: [f32; 4] = [0.15, 0.22, 0.23, 1.];
const MUTED: [f32; 4] = [0.41, 0.47, 0.46, 1.];
const PAPER: [f32; 4] = [0.98, 0.97, 0.94, 1.];
const WHITE: [f32; 4] = [1., 1., 0.99, 1.];
const GREEN: [f32; 4] = [0.19, 0.37, 0.31, 1.];
const GOLD: [f32; 4] = [0.77, 0.43, 0.20, 1.];
const LINE: [f32; 4] = [0.84, 0.85, 0.80, 1.];
#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Map,
    Aerial,
    Street,
    Album,
}
#[derive(Clone, Copy, Debug)]
enum Act {
    Region(usize),
    Select(usize),
    Visit,
    Street,
    World,
    Zoom(f32),
    Mode(Mode),
    Capture,
    Back,
    Turn(f32),
    Search,
    Prev(i32),
    Page(i32),
    Export,
    Import,
    Help,
    Voice,
    Close,
    Photo(usize),
    Year(i32),
    Powers,
}
struct Button {
    rect: [f32; 4],
    act: Act,
}
#[derive(Clone, Debug, PartialEq)]
struct Photo {
    event: usize,
    heading: f32,
    zoom: f32,
    time: f32,
    quality: u8,
}
#[derive(Clone, Debug)]
struct Journal {
    year: i32,
    region: usize,
    selected: usize,
    photos: Vec<Photo>,
}
impl Journal {
    fn new() -> Self {
        Self {
            year: -221,
            region: 0,
            selected: 4,
            photos: vec![],
        }
    }
    fn bytes(&self) -> Vec<u8> {
        let mut b = b"YYHIST\x01\0".to_vec();
        b.extend_from_slice(&self.year.to_le_bytes());
        b.push(self.region as u8);
        b.push(self.selected as u8);
        b.extend_from_slice(&(self.photos.len() as u16).to_le_bytes());
        for p in &self.photos {
            let id = EVENTS[p.event].id.as_bytes();
            b.push(id.len() as u8);
            b.extend_from_slice(id);
            for f in [p.heading, p.zoom, p.time] {
                b.extend_from_slice(&f.to_le_bytes());
            }
            b.push(p.quality);
        }
        let hash = checksum(&b);
        b.extend_from_slice(&hash.to_le_bytes());
        b
    }
    fn read(b: &[u8]) -> Result<Self, String> {
        let invalid =
            || "YYHistoryの記録として読み込めません。現在の写真帳はそのままです。".to_string();
        if b.len() < 20 || b.len() > 65536 || &b[..8] != b"YYHIST\x01\0" {
            return Err(invalid());
        }
        let payload = b.len() - 4;
        if checksum(&b[..payload]) != u32::from_le_bytes(b[payload..].try_into().unwrap()) {
            return Err(invalid());
        }
        let year = i32::from_le_bytes(b[8..12].try_into().unwrap());
        let region = b[12] as usize;
        let selected = b[13] as usize;
        if !(-3000..=2000).contains(&year)
            || year == 0
            || region >= REGIONS.len()
            || selected >= EVENTS.len()
        {
            return Err(invalid());
        }
        let n = u16::from_le_bytes(b[14..16].try_into().unwrap()) as usize;
        if n > EVENTS.len() {
            return Err(invalid());
        }
        let mut at = 16;
        let mut photos = vec![];
        for _ in 0..n {
            if at >= payload {
                return Err(invalid());
            }
            let len = b[at] as usize;
            at += 1;
            if at + len + 13 > payload {
                return Err(invalid());
            }
            let id = std::str::from_utf8(&b[at..at + len]).map_err(|_| invalid())?;
            at += len;
            let event = EVENTS.iter().position(|e| e.id == id).ok_or_else(invalid)?;
            let mut f = [0.; 3];
            for v in &mut f {
                *v = f32::from_le_bytes(b[at..at + 4].try_into().unwrap());
                at += 4;
            }
            let quality = b[at];
            at += 1;
            if !f.iter().all(|v| v.is_finite())
                || f[0].abs() > 180.
                || !(0.7..=2.).contains(&f[1])
                || !(0. ..=1e7).contains(&f[2])
                || !(1..=3).contains(&quality)
                || photos.iter().any(|p: &Photo| p.event == event)
            {
                return Err(invalid());
            }
            photos.push(Photo {
                event,
                heading: f[0],
                zoom: f[1],
                time: f[2],
                quality,
            });
        }
        if at != payload {
            return Err(invalid());
        }
        Ok(Self {
            year,
            region,
            selected,
            photos,
        })
    }
}
fn checksum(b: &[u8]) -> u32 {
    b.iter()
        .fold(2166136261u32, |h, v| (h ^ *v as u32).wrapping_mul(16777619))
}

pub struct Game {
    gfx: Gfx,
    world: Texture,
    journal: Journal,
    mode: Mode,
    previous: Mode,
    buttons: Vec<Button>,
    center: (f32, f32),
    span: f32,
    mouse: (f32, f32),
    down: Option<(f32, f32)>,
    moved: f32,
    heading: f32,
    lens: f32,
    clock: f32,
    flash: f32,
    toast: String,
    toast_left: f32,
    query: String,
    searching: bool,
    list_offset: usize,
    page: usize,
    help: bool,
    voice: bool,
    powers: bool,
    preview: Option<usize>,
    dirty: bool,
    save_in: f32,
    thumbnails: Vec<(Photo, u32, Bitmap)>,
}
impl Game {
    pub fn set_query(&mut self, query: String) {
        self.query = query.chars().take(60).collect();
        self.list_offset = 0;
    }
    pub fn search_visible(&self) -> bool {
        !self.help
    }
    pub fn new(gfx: Gfx) -> Self {
        let image = image::load_from_memory(include_bytes!("../assets/world.png"))
            .expect("offline map")
            .into_rgba8();
        let (w, h) = image.dimensions();
        let data: Vec<u32> = image
            .as_raw()
            .chunks_exact(4)
            .map(|p| u32::from_le_bytes([p[0], p[1], p[2], p[3]]))
            .collect();
        let world = gfx.create_texture(w, h, &data).expect("world texture");
        let mut g = Self {
            gfx,
            world,
            journal: Journal::new(),
            mode: Mode::Map,
            previous: Mode::Map,
            buttons: vec![],
            center: (15., 0.),
            span: 360.,
            mouse: (0., 0.),
            down: None,
            moved: 0.,
            heading: 0.,
            lens: 1.,
            clock: 0.,
            flash: 0.,
            toast: String::new(),
            toast_left: 0.,
            query: String::new(),
            searching: false,
            list_offset: 0,
            page: 0,
            help: false,
            voice: platform::voice_enabled(),
            powers: true,
            preview: None,
            dirty: false,
            save_in: 0.,
            thumbnails: vec![],
        };
        platform::load_slot(0);
        g.notify("地図のピンか年表から出来事を選び、「上空へ」で旅を始めよう。");
        g
    }
    pub fn state_text(&self) -> String {
        format!(
            "YYHistory {:?} {} region={} event={} photos={}",
            self.mode,
            self.journal.year,
            self.journal.region,
            EVENTS[self.journal.selected].id,
            self.journal.photos.len()
        )
    }
    pub fn crash_bytes(&self) -> Option<Vec<u8>> {
        Some(self.journal.bytes())
    }
    pub fn notify(&mut self, s: &str) {
        self.toast = s.to_string();
        self.toast_left = 5.;
    }
    pub fn apply_loaded(&mut self, b: &[u8]) {
        match Journal::read(b) {
            Ok(j) => {
                self.journal = j;
                self.thumbnails.clear();
                self.fit_region();
                self.mode = Mode::Map;
                self.preview = None;
                self.notify("写真帳を読み込みました。");
            }
            Err(e) => self.notify(&e),
        }
    }
    fn save(&mut self) {
        platform::save_slot(
            0,
            &self.journal.bytes(),
            &format!(
                "YYHistory {} / {}枚",
                year_text(self.journal.year),
                self.journal.photos.len()
            ),
        );
        self.dirty = false;
    }
    fn change(&mut self) {
        self.dirty = true;
        self.save_in = 0.8;
    }
    fn sidebar(&self) -> f32 {
        if self.gfx.compact {
            270.
        } else {
            310.
        }
    }
    fn area(&self) -> [f32; 4] {
        [
            self.sidebar(),
            72.,
            self.gfx.width as f32 - self.sidebar(),
            self.gfx.height as f32 - 256.,
        ]
    }
    fn fit_region(&mut self) {
        let r = &REGIONS[self.journal.region];
        self.center = (r.lon, r.lat);
        self.span = r.span;
    }
    fn filtered(&self) -> Vec<usize> {
        let mut v = matching(self.journal.region, &self.query);
        v.sort_by_key(|i| EVENTS[*i].year);
        v
    }
    fn found(&self, i: usize) -> bool {
        self.journal.photos.iter().any(|p| p.event == i)
    }
    fn select(&mut self, i: usize) {
        self.journal.selected = i;
        self.journal.year = EVENTS[i].year;
        self.heading = 0.;
        self.lens = 1.;
        self.preview = None;
        self.change();
        if self.voice {
            platform::speak(&format!(
                "{}。{}。{}",
                year_text(EVENTS[i].year),
                EVENTS[i].title,
                EVENTS[i].body
            ));
        }
    }
    fn set_year(&mut self, y: i32) {
        self.journal.year = if y == 0 { 1 } else { y.clamp(-3000, 2000) };
        self.change();
    }
    fn map_scale(&self) -> f32 {
        let [_, _, w, h] = self.area();
        (w / self.span).min((h - 170.).max(120.) / (self.span * 0.5))
    }
    fn project(&self, lon: f32, lat: f32) -> (f32, f32) {
        let [x, y, w, h] = self.area();
        let scale = self.map_scale();
        let dx = (lon - self.center.0 + 180.).rem_euclid(360.) - 180.;
        (
            x + w / 2. + dx * scale,
            y + (h - 170.) / 2. - (lat - self.center.1) * scale,
        )
    }
    fn zoom(&mut self, f: f32) {
        self.span = (self.span * f).clamp(8., 360.);
    }
    pub fn hit_kind(&self, x: f32, y: f32) -> u32 {
        let (x, y) = (x / self.gfx.ui, y / self.gfx.ui);
        if self.help
            || self.buttons.iter().any(|b| inside(b.rect, x, y))
            || y < 72.
            || y > self.gfx.height as f32 - 184.
        {
            1
        } else if x < self.sidebar() {
            3
        } else {
            0
        }
    }
    pub fn frame(&mut self, evs: &[Ev], dt: f32, _focused: bool) -> bool {
        if let Some(b) = platform::take_loaded() {
            self.apply_loaded(&b);
        }
        for n in platform::take_notes() {
            self.notify(&n);
        }
        self.clock = (self.clock + dt).min(1e7);
        self.flash = (self.flash - dt * 2.).max(0.);
        self.toast_left -= dt;
        for e in evs {
            self.handle(*e);
        }
        if self.dirty {
            self.save_in -= dt;
            if self.save_in <= 0. {
                self.save();
            }
        }
        self.render();
        false
    }
    fn act(&mut self, a: Act) {
        match a {
            Act::Region(r) => {
                self.journal.region = r;
                self.fit_region();
                self.mode = Mode::Map;
                self.list_offset = 0;
                self.page = 0;
                self.preview = None;
                let v = self.filtered();
                if !v.contains(&self.journal.selected) {
                    if let Some(i) = v
                        .iter()
                        .min_by_key(|i| (EVENTS[**i].year - self.journal.year).abs())
                    {
                        self.select(*i);
                    }
                }
                self.change();
            }
            Act::Select(i) => {
                self.select(i);
                if self.mode == Mode::Album {
                    self.mode = Mode::Map;
                }
            }
            Act::Visit => {
                self.mode = Mode::Aerial;
                self.preview = None;
                self.journal.year = EVENTS[self.journal.selected].year;
                self.heading = 0.;
                self.lens = 1.;
                self.change();
            }
            Act::Street => {
                self.mode = Mode::Street;
                self.preview = None;
                self.heading = 0.;
                self.lens = 1.;
            }
            Act::World => {
                self.mode = Mode::Map;
                self.journal.region = 0;
                self.fit_region();
                self.list_offset = 0;
                self.change();
            }
            Act::Mode(m) => {
                self.mode = m;
                self.preview = None;
                self.page = 0;
            }
            Act::Zoom(f) => {
                if self.mode == Mode::Map {
                    self.zoom(f);
                } else {
                    self.lens = (self.lens / f).clamp(0.7, 2.);
                }
            }
            Act::Back => {
                if self.preview.take().is_none() {
                    self.mode = match self.mode {
                        Mode::Street => Mode::Aerial,
                        Mode::Aerial | Mode::Album => Mode::Map,
                        Mode::Map => Mode::Map,
                    };
                }
            }
            Act::Turn(d) => {
                self.heading = (self.heading + d + 180.).rem_euclid(360.) - 180.;
            }
            Act::Capture => self.capture(),
            Act::Search => self.searching = true,
            Act::Prev(d) => {
                let v = self.filtered();
                if !v.is_empty() {
                    let at = v
                        .iter()
                        .position(|i| *i == self.journal.selected)
                        .unwrap_or(0) as i32;
                    self.select(v[(at + d).rem_euclid(v.len() as i32) as usize]);
                }
            }
            Act::Page(d) => {
                self.page = (self.page as i32 + d).max(0) as usize;
            }
            Act::Export => {
                self.save();
                platform::export_slot(0);
            }
            Act::Import => platform::import_file(),
            Act::Help => {
                self.help = true;
            }
            Act::Close => {
                self.help = false;
                self.preview = None;
            }
            Act::Voice => {
                self.voice = !self.voice;
                platform::save_voice(self.voice);
                if self.voice {
                    platform::speak(EVENTS[self.journal.selected].body);
                } else {
                    platform::speak("");
                }
            }
            Act::Photo(i) => {
                self.previous = self.mode;
                self.preview = Some(i);
            }
            Act::Year(y) => self.set_year(y),
            Act::Powers => self.powers = !self.powers,
        }
    }
    fn capture(&mut self) {
        if self.mode != Mode::Street || self.preview.is_some() {
            self.notify("出来事の現場に降りて、風景を撮影しよう。");
            return;
        }
        if self.heading.abs() > 70. {
            self.notify("目標がフレームの外です。矢印で正面（0°）へ向き直ろう。");
            return;
        }
        let quality = if self.heading.abs() < 18. && self.lens >= 1. {
            3
        } else if self.heading.abs() < 42. {
            2
        } else {
            1
        };
        let p = Photo {
            event: self.journal.selected,
            heading: self.heading,
            zoom: self.lens,
            time: self.clock,
            quality,
        };
        if let Some(old) = self
            .journal
            .photos
            .iter_mut()
            .find(|old| old.event == p.event)
        {
            if old.quality <= quality {
                *old = p;
                self.notify("写真を更新しました。地域別年表に保存しました。");
            } else {
                self.notify("撮影しました。写真帳には、よりよく撮れた写真を残しました。");
            }
        } else {
            self.journal.photos.push(p);
            self.notify("新しい歴史写真！ 地域別年表に収録しました。");
        }
        self.flash = 1.;
        self.save();
    }
    fn handle(&mut self, e: Ev) {
        let ui = self.gfx.ui;
        match e {
            Ev::Resize(w, h) => self.gfx.resize(w.max(1), h.max(1)),
            Ev::Compact(c) => self.gfx.set_compact(c),
            Ev::Down(0, x, y) => {
                self.mouse = (x / ui, y / ui);
                self.down = Some(self.mouse);
                self.moved = 0.;
            }
            Ev::MouseMove(x, y) => {
                let (x, y) = (x / ui, y / ui);
                let (dx, dy) = (x - self.mouse.0, y - self.mouse.1);
                if let Some((sx, sy)) = self.down {
                    self.moved += dx.abs() + dy.abs();
                    let a = self.area();
                    if !self.help
                        && sx > a[0]
                        && sy > a[1] + 70.
                        && sy < a[1] + a[3] - 65.
                        && self.preview.is_none()
                    {
                        if self.mode == Mode::Map {
                            let scale = self.map_scale();
                            self.center.0 =
                                (self.center.0 - dx / scale + 180.).rem_euclid(360.) - 180.;
                            self.center.1 = (self.center.1 + dy / scale).clamp(-70., 80.);
                        } else if self.mode == Mode::Street {
                            self.heading =
                                (self.heading - dx * 0.25 + 180.).rem_euclid(360.) - 180.;
                        }
                    } else if sy > self.gfx.height as f32 - 69. && sy < self.gfx.height as f32 - 35.
                    {
                        self.slider(x);
                    }
                }
                self.mouse = (x, y);
            }
            Ev::Up(0, x, y) => {
                let (x, y) = (x / ui, y / ui);
                if self.down.take().is_some() && self.moved < 7. {
                    if let Some(act) = self
                        .buttons
                        .iter()
                        .rev()
                        .find(|b| inside(b.rect, x, y))
                        .map(|b| b.act)
                    {
                        self.searching = false;
                        self.act(act);
                    } else if !self.help {
                        self.searching = false;
                        if y > self.gfx.height as f32 - 69. && y < self.gfx.height as f32 - 35. {
                            self.slider(x);
                        } else if self.mode == Mode::Map && x > self.sidebar() {
                            let mut nearest = None;
                            let mut dist = 22.;
                            for i in self.filtered() {
                                let e = &EVENTS[i];
                                let (px, py) = self.project(e.lon, e.lat);
                                let dd = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
                                if dd < dist {
                                    nearest = Some(i);
                                    dist = dd;
                                }
                            }
                            if let Some(i) = nearest {
                                self.select(i);
                            }
                        }
                    }
                }
            }
            Ev::Wheel(d) => {
                if self.help {
                    return;
                }
                if self.mouse.0 < self.sidebar() {
                    let n = self.filtered().len();
                    self.list_offset = (self.list_offset as i32 - if d > 0. { 1 } else { -1 })
                        .clamp(0, n.saturating_sub(1) as i32)
                        as usize;
                } else if self.mode == Mode::Album {
                    self.page = (self.page as i32 - if d > 0. { 1 } else { -1 })
                        .clamp(0, self.album_pages().saturating_sub(1) as i32)
                        as usize;
                } else {
                    self.act(Act::Zoom(if d > 0. { 0.8 } else { 1.25 }));
                }
            }
            Ev::Scroll(d) => {
                if self.mouse.0 < self.sidebar() {
                    let n = self.filtered().len();
                    self.list_offset = (self.list_offset as i32 + (d / 25.) as i32)
                        .clamp(0, n.saturating_sub(1) as i32)
                        as usize;
                }
            }
            Ev::Char(c) => {
                if self.searching && !c.is_control() && self.query.chars().count() < 60 {
                    self.query.push(c);
                    self.list_offset = 0;
                }
            }
            Ev::Key(k, true) => {
                if self.searching {
                    match k {
                        8 => {
                            self.query.pop();
                            self.list_offset = 0;
                        }
                        13 | 27 => self.searching = false,
                        _ => (),
                    }
                    return;
                }
                if self.help {
                    if k == 27 || k == 13 {
                        self.help = false;
                    }
                    return;
                }
                match k {
                    27 => self.act(Act::Back),
                    13 => self.act(if self.mode == Mode::Aerial {
                        Act::Street
                    } else {
                        Act::Visit
                    }),
                    32 => self.capture(),
                    37 | 65 => self.act(Act::Turn(-15.)),
                    39 | 68 => self.act(Act::Turn(15.)),
                    38 | 87 => self.act(Act::Zoom(0.9)),
                    40 | 83 => self.act(Act::Zoom(1.1)),
                    0xbb | 0x6b => self.act(Act::Zoom(0.8)),
                    0xbd | 0x6d => self.act(Act::Zoom(1.25)),
                    0xbc => self.act(Act::Prev(-1)),
                    0xbe => self.act(Act::Prev(1)),
                    0x71 => self.act(Act::Mode(Mode::Album)),
                    0x72 => self.help = true,
                    0x7a => self.gfx.toggle_fullscreen(),
                    _ => (),
                }
            }
            _ => (),
        }
    }
    fn slider(&mut self, x: f32) {
        let start = self.sidebar() + 92.;
        let end = self.gfx.width as f32 - 40.;
        if end > start {
            let t = ((x - start) / (end - start)).clamp(0., 1.);
            self.set_year((-3000. + t * 5000.).round() as i32);
        }
    }
    fn label(&mut self, s: &str, x: f32, y: f32, w: f32, h: f32, font: Font, c: [f32; 4]) {
        self.gfx.text(s, x, y, w, h, font, c);
    }
    fn button(&mut self, s: &str, r: [f32; 4], a: Act, active: bool) {
        let [x, y, w, h] = r;
        let hover = inside(r, self.mouse.0, self.mouse.1);
        self.gfx.fill_round(
            x,
            y,
            x + w,
            y + h,
            7.,
            if active {
                GREEN
            } else if hover {
                [0.90, 0.92, 0.87, 1.]
            } else {
                WHITE
            },
        );
        self.gfx.stroke_round(
            x,
            y,
            x + w,
            y + h,
            7.,
            1.,
            if active { GREEN } else { LINE },
        );
        self.label(
            s,
            x + 6.,
            y + 2.,
            w - 12.,
            h - 4.,
            Font::NormalCenter,
            if active { WHITE } else { INK },
        );
        self.buttons.push(Button { rect: r, act: a });
    }
    fn render(&mut self) {
        self.buttons.clear();
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let side = self.sidebar();
        let area = self.area();
        self.gfx.begin(PAPER);
        self.gfx.set_screen_xform();
        for p in &self.journal.photos {
            if !self
                .thumbnails
                .iter()
                .any(|(q, gen, _)| q == p && *gen == self.gfx.d2d_gen)
            {
                let pixels =
                    scene::thumbnail(&mut self.gfx, &EVENTS[p.event], p.heading, p.zoom, p.time);
                if let Some(bmp) = self.gfx.create_bitmap(320, 180, &pixels) {
                    self.thumbnails.retain(|(q, _, _)| q.event != p.event);
                    self.thumbnails.push((p.clone(), self.gfx.d2d_gen, bmp));
                }
            }
        }
        match self.mode {
            Mode::Map => self.map_geometry(),
            Mode::Aerial | Mode::Street => scene::draw(
                &mut self.gfx,
                &EVENTS[self.journal.selected],
                area,
                self.mode == Mode::Aerial,
                self.heading,
                self.lens,
                self.clock,
            ),
            Mode::Album => self.album_geometry(),
        }
        if let Some(i) = self.preview {
            let p = &self.journal.photos[i];
            scene::draw(
                &mut self.gfx,
                &EVENTS[p.event],
                [side + 24., 132., w - side - 48., h - 380.],
                false,
                p.heading,
                p.zoom,
                p.time,
            );
        }
        self.gfx.flush();
        if !self.gfx.d2d_begin() {
            return;
        }
        self.gfx.fill_rect(0., 0., w, 72., PAPER);
        self.gfx.fill_rect(0., 72., side, h, PAPER);
        self.gfx.fill_rect(side, h - 184., w, h, PAPER);
        self.gfx.stroke_rect(side, 72., side + 1., h, 1., LINE);
        self.gfx.stroke_rect(0., 71., w, 72., 1., LINE);
        self.label("YYHistory", 24., 13., 220., 34., Font::TitleMid, GREEN);
        self.label(
            "時を歩く、歴史を撮る",
            27.,
            48.,
            230.,
            17.,
            Font::Small,
            MUTED,
        );
        self.button(
            "地図",
            [side + 22., 19., 64., 34.],
            Act::Mode(Mode::Map),
            self.mode == Mode::Map,
        );
        self.button(
            "写真帳",
            [side + 94., 19., 86., 34.],
            Act::Mode(Mode::Album),
            self.mode == Mode::Album,
        );
        self.label(
            &format!(
                "{} / {} の出来事を記録",
                self.journal.photos.len(),
                EVENTS.len()
            ),
            side + 202.,
            25.,
            260.,
            25.,
            Font::Normal,
            MUTED,
        );
        self.button("書き出し", [w - 336., 19., 88., 34.], Act::Export, false);
        self.button("読み込み", [w - 240., 19., 88., 34.], Act::Import, false);
        self.button("使い方", [w - 144., 19., 72., 34.], Act::Help, self.help);
        self.button(
            if self.voice { "声 ON" } else { "声 OFF" },
            [w - 65., 19., 60., 34.],
            Act::Voice,
            self.voice,
        );
        self.sidebar_ui();
        if self.preview.is_some() {
            self.preview_ui();
        } else {
            match self.mode {
                Mode::Map => self.map_ui(),
                Mode::Aerial | Mode::Street => self.scene_ui(),
                Mode::Album => self.album_ui(),
            }
        }
        self.timeline();
        if self.toast_left > 0. {
            let tw = (w - side - 160.).min(650.);
            let tx = side + (w - side - tw) / 2.;
            self.gfx.fill_round(
                tx,
                h - 232.,
                tx + tw,
                h - 193.,
                8.,
                [0.13, 0.24, 0.22, 0.95],
            );
            self.label(
                &self.toast.clone(),
                tx + 12.,
                h - 225.,
                tw - 24.,
                26.,
                Font::NormalCenter,
                WHITE,
            );
        }
        if self.flash > 0. {
            self.gfx
                .fill_rect(side, 72., w, h - 184., [1., 1., 1., self.flash * 0.65]);
        }
        if self.help {
            self.help_ui();
        }
        self.gfx.d2d_end();
        self.gfx.present();
    }
    fn sidebar_ui(&mut self) {
        let side = self.sidebar();
        let h = self.gfx.height as f32;
        self.label(
            "歴史を探しにいこう",
            20.,
            88.,
            side - 40.,
            28.,
            Font::Big,
            INK,
        );
        let prompt = if self.query.is_empty() {
            if self.searching {
                "入力してください…"
            } else {
                "地名・出来事・年代で検索"
            }
        } else {
            &self.query
        };
        self.button(
            &prompt.to_string(),
            [18., 127., side - 36., 39.],
            Act::Search,
            self.searching,
        );
        for r in 0..REGIONS.len() {
            let x = 18. + (r % 2) as f32 * (side - 30.) / 2.;
            let y = 180. + (r / 2) as f32 * 37.;
            self.button(
                match r {
                    4 => "中東・北アフリカ",
                    7 => "アフリカ・大洋州",
                    _ => REGIONS[r].name,
                },
                [x, y, (side - 42.) / 2., 30.],
                Act::Region(r),
                self.journal.region == r,
            );
        }
        let v = self.filtered();
        self.label(
            &format!("地域別年表  /  {}件", v.len()),
            20.,
            335.,
            side - 40.,
            28.,
            Font::Big,
            INK,
        );
        let end = h - 233.;
        let rows = ((end - 375.) / 58.).max(1.) as usize;
        self.list_offset = self.list_offset.min(v.len().saturating_sub(rows));
        for (k, i) in v.iter().skip(self.list_offset).take(rows).enumerate() {
            let e = &EVENTS[*i];
            let y = 374. + k as f32 * 58.;
            let active = *i == self.journal.selected;
            if active {
                self.gfx
                    .fill_round(12., y - 3., side - 12., y + 50., 7., [0.88, 0.92, 0.85, 1.]);
            }
            self.gfx
                .fill_circle(25., y + 14., 4., if self.found(*i) { GOLD } else { LINE });
            self.label(
                &year_text(e.year),
                39.,
                y,
                side - 60.,
                18.,
                Font::Small,
                if active { GREEN } else { MUTED },
            );
            self.label(
                e.title,
                39.,
                y + 20.,
                side - if self.found(*i) { 130. } else { 60. },
                27.,
                Font::Normal,
                INK,
            );
            if let Some((_, _, bmp)) = self.thumbnails.iter().find(|(p, _, _)| p.event == *i) {
                self.gfx
                    .draw_bitmap(bmp, side - 76., y + 7., side - 20., y + 39.);
            }
            self.buttons.push(Button {
                rect: [12., y - 3., side - 24., 53.],
                act: Act::Select(*i),
            });
        }
        if v.is_empty() {
            self.label(
                "該当する出来事がありません。\n検索語や地域を変えてください。",
                24.,
                385.,
                side - 48.,
                90.,
                Font::Normal,
                MUTED,
            );
        }
        self.label(
            "スクロールで年表をたどる",
            22.,
            h - 222.,
            side - 44.,
            23.,
            Font::Small,
            MUTED,
        );
        self.gfx
            .fill_rect(16., h - 188., side - 16., h - 187., LINE);
        self.label(
            "あなたの歴史写真帳",
            22.,
            h - 171.,
            side - 44.,
            25.,
            Font::Big,
            GREEN,
        );
        let done = EVENTS
            .iter()
            .enumerate()
            .filter(|(i, e)| {
                (self.journal.region == 0 || e.region == self.journal.region) && self.found(*i)
            })
            .count();
        let total = EVENTS
            .iter()
            .filter(|e| self.journal.region == 0 || e.region == self.journal.region)
            .count();
        self.label(
            &format!(
                "{} / {} 枚  ·  {}",
                done, total, REGIONS[self.journal.region].name
            ),
            22.,
            h - 138.,
            side - 44.,
            29.,
            Font::Normal,
            MUTED,
        );
        self.gfx
            .fill_round(22., h - 99., side - 22., h - 93., 3., LINE);
        if total > 0 {
            self.gfx.fill_round(
                22.,
                h - 99.,
                22. + (side - 44.) * done as f32 / total as f32,
                h - 93.,
                3.,
                GREEN,
            );
        }
        self.button(
            "写真を並べて見る",
            [20., h - 72., side - 40., 38.],
            Act::Mode(Mode::Album),
            self.mode == Mode::Album,
        );
        self.label(
            "撮った写真は自動で保存されます",
            22.,
            h - 27.,
            side - 44.,
            18.,
            Font::Small,
            MUTED,
        );
    }
    fn map_geometry(&mut self) {
        let [x, y, w, h] = self.area();
        let scale = self.map_scale();
        let px = x + w / 2. - (180. + self.center.0) * scale;
        let py = y + (h - 170.) / 2. - (90. - self.center.1) * scale;
        self.gfx.rect(x, y, x + w, y + h, [0.66, 0.81, 0.85, 1.]);
        for k in if self.span >= 300. { 0..=0 } else { -1..=1 } {
            let xx = px + k as f32 * 360. * scale;
            self.gfx.draw_textured(
                &self.world,
                xx,
                py,
                xx + 360. * scale,
                py + 180. * scale,
                [1., 1., 1., 1.],
                true,
            );
        }
        if self.powers {
            for p in POWERS
                .iter()
                .filter(|p| p.from <= self.journal.year && p.to >= self.journal.year)
            {
                let (cx, cy) = self.project(p.lon, p.lat);
                let mut prev = None;
                for n in 0..=64 {
                    let a = n as f32 / 64. * std::f32::consts::TAU;
                    let pt = (cx + a.cos() * p.rx * scale, cy + a.sin() * p.ry * scale);
                    if let Some(last) = prev {
                        self.gfx.tri((cx, cy), last, pt, p.color);
                        self.gfx.line(
                            last,
                            pt,
                            1.2,
                            [
                                p.color[0] * 0.65,
                                p.color[1] * 0.65,
                                p.color[2] * 0.65,
                                0.65,
                            ],
                        );
                    }
                    prev = Some(pt);
                }
            }
        }
    }
    fn map_ui(&mut self) {
        let [x, y, w, h] = self.area();
        let right = x + w;
        let bottom = y + h;
        // Keep map drawing within its panel, including during deep zoom.
        self.gfx
            .fill_round(x + 20., y + 18., x + 258., y + 68., 9., WHITE);
        self.label(
            &year_text(self.journal.year),
            x + 35.,
            y + 26.,
            215.,
            30.,
            Font::Big,
            GREEN,
        );
        self.button(
            "世界に戻る",
            [right - 146., y + 18., 126., 34.],
            Act::World,
            false,
        );
        self.button(
            if self.powers {
                "勢力 ON"
            } else {
                "勢力 OFF"
            },
            [right - 146., y + 60., 126., 34.],
            Act::Powers,
            self.powers,
        );
        let year = self.journal.year;
        if self.powers {
            for p in POWERS.iter().filter(|p| p.from <= year && p.to >= year) {
                let (px, py) = self.project(p.lon, p.lat);
                if px > x + 60. && px < right - 70. && py > y + 12. && py < bottom - 170. {
                    self.label(
                        p.name,
                        px - 85.,
                        py - 13.,
                        170.,
                        24.,
                        Font::NormalCenter,
                        GREEN,
                    );
                }
            }
        }
        let v = self.filtered();
        let mut positions: Vec<(f32, f32)> = vec![];
        // Selected marker goes last. Nearby markers use small deterministic offsets.
        let mut order: Vec<usize> = v
            .iter()
            .copied()
            .filter(|i| *i != self.journal.selected)
            .collect();
        if v.contains(&self.journal.selected) {
            order.push(self.journal.selected);
        }
        for i in order {
            let e = &EVENTS[i];
            let (px, mut py) = self.project(e.lon, e.lat);
            if px < x + 18. || px > right - 18. || py < y + 12. || py > bottom - 165. {
                continue;
            }
            let active = i == self.journal.selected;
            if !active {
                while positions
                    .iter()
                    .any(|(a, b)| (a - px).abs() < 19. && (b - py).abs() < 20.)
                {
                    py += 21.;
                }
            }
            positions.push((px, py));
            let near = (e.year - self.journal.year).abs() < 100;
            let c = if active {
                GOLD
            } else if self.found(i) {
                GREEN
            } else if near {
                [0.35, 0.47, 0.40, 1.]
            } else {
                [0.57, 0.63, 0.57, 0.85]
            };
            self.gfx
                .fill_circle(px, py, if active { 11. } else { 7. }, WHITE);
            self.gfx
                .fill_circle(px, py, if active { 8. } else { 5. }, c);
            self.buttons.push(Button {
                rect: [px - 11., py - 11., 22., 22.],
                act: Act::Select(i),
            });
            if active {
                self.gfx
                    .fill_round(px + 14., py - 16., px + 199., py + 34., 6., WHITE);
                self.label(e.title, px + 23., py - 10., 168., 23., Font::Normal, INK);
                self.label(e.place, px + 23., py + 13., 168., 18., Font::Small, MUTED);
            }
        }
        self.button(
            "＋",
            [right - 62., bottom - 270., 42., 42.],
            Act::Zoom(0.75),
            false,
        );
        self.button(
            "−",
            [right - 62., bottom - 221., 42., 42.],
            Act::Zoom(1.333),
            false,
        );
        let scale_km = 111. * 100. / self.map_scale();
        self.label(
            &format!("約 {:0.0} km  ━━━", scale_km),
            x + 24.,
            bottom - 185.,
            210.,
            23.,
            Font::Small,
            MUTED,
        );
        self.event_card(x + 20., bottom - 157., w - 40.);
        self.label("Natural Earth · 正距円筒図法 / 勢力の楕円は交流・影響圏の概略（領土境界ではありません）",x+20.,bottom-22.,w-40.,18.,Font::Small,MUTED);
    }
    fn event_card(&mut self, x: f32, y: f32, w: f32) {
        let e = &EVENTS[self.journal.selected];
        self.gfx.fill_round(x, y, x + w, y + 126., 10., WHITE);
        self.label(
            &format!("{}  /  {}  /  {}", year_text(e.year), e.place, e.category),
            x + 18.,
            y + 10.,
            w - 230.,
            21.,
            Font::Small,
            MUTED,
        );
        self.label(e.title, x + 18., y + 35., w - 230., 28., Font::Big, INK);
        self.label(e.body, x + 18., y + 70., w - 245., 48., Font::Normal, MUTED);
        self.button(
            "上空へ  →",
            [x + w - 185., y + 27., 165., 44.],
            Act::Visit,
            true,
        );
        self.label(
            if self.found(self.journal.selected) {
                "● 撮影済み"
            } else {
                "○ 未撮影の出来事"
            },
            x + w - 185.,
            y + 80.,
            165.,
            24.,
            Font::NormalCenter,
            GOLD,
        );
    }
    fn scene_ui(&mut self) {
        let [x, y, w, h] = self.area();
        let e = &EVENTS[self.journal.selected];
        let street = self.mode == Mode::Street;
        self.gfx
            .fill_round(x + 20., y + 18., x + 470., y + 83., 9., WHITE);
        self.label(
            &format!("{}  /  {}", year_text(e.year), e.place),
            x + 36.,
            y + 25.,
            420.,
            20.,
            Font::Small,
            MUTED,
        );
        self.label(e.title, x + 36., y + 47., 420., 29., Font::Big, INK);
        self.button(
            "地図",
            [x + w - 244., y + 18., 64., 34.],
            Act::Mode(Mode::Map),
            false,
        );
        self.button(
            "上空",
            [x + w - 172., y + 18., 64., 34.],
            Act::Mode(Mode::Aerial),
            !street,
        );
        self.button(
            "現場",
            [x + w - 100., y + 18., 80., 34.],
            Act::Street,
            street,
        );
        self.gfx.fill_round(
            x + 20.,
            y + h - 118.,
            x + w - 20.,
            y + h - 20.,
            9.,
            [0.98, 0.97, 0.93, 0.96],
        );
        if street {
            self.label(
                &format!(
                    "現場を観察  /  方位 {:0.0}°  /  レンズ {:0.1}×",
                    self.heading, self.lens
                ),
                x + 36.,
                y + h - 107.,
                w - 430.,
                25.,
                Font::Big,
                GREEN,
            );
            self.label(
                "左右ドラッグ・← → で見回す。正面の出来事をフレームに収めよう。",
                x + 36.,
                y + h - 71.,
                w - 430.,
                42.,
                Font::Normal,
                MUTED,
            );
            self.button(
                "↶",
                [x + w - 383., y + h - 94., 44., 44.],
                Act::Turn(-15.),
                false,
            );
            self.button(
                "↷",
                [x + w - 331., y + h - 94., 44., 44.],
                Act::Turn(15.),
                false,
            );
            self.button(
                "撮影する  [Space]",
                [x + w - 270., y + h - 94., 245., 44.],
                Act::Capture,
                true,
            );
            let (cx, cy) = (x + w * 0.5, y + h * 0.47);
            let cw = w * 0.44;
            let ch = h * 0.49;
            for (sx, sy) in [(-1., -1.), (1., -1.), (-1., 1.), (1., 1.)] {
                let (xx, yy) = (cx + sx * cw * 0.5, cy + sy * ch * 0.5);
                self.gfx.fill_rect(
                    xx.min(xx - sx * 24.),
                    yy - 1.,
                    xx.max(xx - sx * 24.),
                    yy + 1.,
                    WHITE,
                );
                self.gfx.fill_rect(
                    xx - 1.,
                    yy.min(yy - sy * 24.),
                    xx + 1.,
                    yy.max(yy - sy * 24.),
                    WHITE,
                );
            }
            self.gfx.fill_circle(cx, cy, 3., WHITE);
            self.label(
                "HISTORY CAMERA  ·  復元風景",
                x + w * 0.5 - 180.,
                y + 107.,
                360.,
                22.,
                Font::NormalCenter,
                WHITE,
            );
        } else {
            self.label(
                "上空から街を見渡す",
                x + 36.,
                y + h - 107.,
                w - 330.,
                28.,
                Font::Big,
                GREEN,
            );
            self.label(
                "道・建物・水辺をたどって、歴史の現場に降りてみよう。",
                x + 36.,
                y + h - 70.,
                w - 330.,
                42.,
                Font::Normal,
                MUTED,
            );
            self.button(
                "現場に降りる  ↓",
                [x + w - 270., y + h - 94., 245., 44.],
                Act::Street,
                true,
            );
            self.button(
                "広場へ降りる",
                [x + w * 0.38, y + h * 0.38, 150., 38.],
                Act::Street,
                false,
            );
        }
        self.label(
            "年代と地域を参考にしたデフォルメ復元です。実際の航空写真・街路の再現ではありません。",
            x + 20.,
            y + h - 18.,
            w - 40.,
            18.,
            Font::Small,
            MUTED,
        );
    }
    fn timeline(&mut self) {
        let side = self.sidebar();
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let x = side + 22.;
        self.gfx.fill_rect(side, h - 184., w, h, PAPER);
        self.gfx.fill_rect(side, h - 184., w, h - 183., LINE);
        self.label(
            &format!("{} の年表", REGIONS[self.journal.region].name),
            x,
            h - 173.,
            300.,
            24.,
            Font::Big,
            INK,
        );
        self.label(
            "出来事を選ぶと、その年へ時間旅行",
            x + 300.,
            h - 168.,
            360.,
            23.,
            Font::Small,
            MUTED,
        );
        self.button("‹", [w - 100., h - 175., 34., 28.], Act::Prev(-1), false);
        self.button("›", [w - 59., h - 175., 34., 28.], Act::Prev(1), false);
        let v = self.filtered();
        let n = ((w - side - 44.) / 172.).floor().max(1.) as usize;
        let selected = v
            .iter()
            .position(|i| *i == self.journal.selected)
            .unwrap_or(0);
        let start = selected
            .saturating_sub(n / 2)
            .min(v.len().saturating_sub(n));
        let cardw = (w - side - 44.) / n as f32;
        for (k, i) in v.iter().skip(start).take(n).enumerate() {
            let e = &EVENTS[*i];
            let xx = x + k as f32 * cardw;
            let active = *i == self.journal.selected;
            self.gfx.fill_round(
                xx,
                h - 137.,
                xx + cardw - 8.,
                h - 80.,
                6.,
                if active {
                    [0.87, 0.91, 0.84, 1.]
                } else {
                    WHITE
                },
            );
            self.label(
                &format!(
                    "{} {}",
                    if self.found(*i) { "●" } else { "○" },
                    year_text(e.year)
                ),
                xx + 9.,
                h - 132.,
                cardw - 25.,
                19.,
                Font::Small,
                if self.found(*i) { GOLD } else { MUTED },
            );
            self.label(
                e.title,
                xx + 9.,
                h - 110.,
                cardw - if self.found(*i) { 80. } else { 25. },
                26.,
                Font::Normal,
                INK,
            );
            if let Some((_, _, bmp)) = self.thumbnails.iter().find(|(p, _, _)| p.event == *i) {
                self.gfx
                    .draw_bitmap(bmp, xx + cardw - 64., h - 130., xx + cardw - 14., h - 102.);
            }
            self.buttons.push(Button {
                rect: [xx, h - 137., cardw - 8., 57.],
                act: Act::Select(*i),
            });
        }
        self.label("時代", x, h - 62., 60., 25., Font::Normal, GREEN);
        let a = side + 92.;
        let b = w - 40.;
        let yy = h - 49.;
        self.gfx.fill_rect(a, yy - 2., b, yy + 2., LINE);
        let marks = [
            (-3000, "紀元前3000"),
            (-2000, "2000"),
            (-1000, "1000"),
            (1, "紀元"),
            (1000, "1000"),
            (2000, "2000年"),
        ];
        for (year, label) in marks {
            let xx = a + (year + 3000) as f32 / 5000. * (b - a);
            self.gfx.fill_rect(xx, yy - 6., xx + 1., yy + 6., MUTED);
            self.label(label, xx - 38., h - 32., 76., 18., Font::Small, MUTED);
            self.buttons.push(Button {
                rect: [xx - 10., yy - 10., 20., 20.],
                act: Act::Year(year),
            });
        }
        let t = (self.journal.year + 3000) as f32 / 5000.;
        let xx = a + t * (b - a);
        self.gfx.fill_circle(xx, yy, 8., GREEN);
        self.label(
            &year_text(self.journal.year),
            xx - 65.,
            h - 77.,
            130.,
            20.,
            Font::NormalCenter,
            GREEN,
        );
    }
    fn album_events(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self
            .journal
            .photos
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                self.journal.region == 0 || EVENTS[p.event].region == self.journal.region
            })
            .map(|(i, _)| i)
            .collect();
        v.sort_by_key(|i| EVENTS[self.journal.photos[*i].event].year);
        v
    }
    fn album_pages(&self) -> usize {
        self.album_events().len().div_ceil(6).max(1)
    }
    fn album_geometry(&mut self) {
        let [x, y, w, h] = self.area();
        self.gfx.rect(x, y, x + w, y + h, PAPER);
        let ids = self.album_events();
        self.page = self.page.min(self.album_pages() - 1);
        let cw = (w - 72.) / 3.;
        let ch = (h - 140.) / 2.;
        for (k, i) in ids.iter().skip(self.page * 6).take(6).enumerate() {
            let p = &self.journal.photos[*i];
            let xx = x + 24. + (k % 3) as f32 * (cw + 12.);
            let yy = y + 90. + (k / 3) as f32 * (ch + 12.);
            scene::draw(
                &mut self.gfx,
                &EVENTS[p.event],
                [xx + 7., yy + 7., cw - 14., ch - 64.],
                false,
                p.heading,
                p.zoom,
                p.time,
            );
        }
    }
    fn album_ui(&mut self) {
        let [x, y, w, h] = self.area();
        self.label(
            "旅の記憶を、一枚ずつ。",
            x + 24.,
            y + 21.,
            w - 250.,
            35.,
            Font::TitleMid,
            GREEN,
        );
        self.label(
            "撮影した復元風景を、地域と年代順に並べたあなたの写真帳",
            x + 27.,
            y + 59.,
            w - 280.,
            25.,
            Font::Normal,
            MUTED,
        );
        let ids = self.album_events();
        if ids.is_empty() {
            self.label(
                "まだ、この地域の写真はありません。",
                x + 50.,
                y + h * 0.37,
                w - 100.,
                40.,
                Font::BigCenter,
                GREEN,
            );
            self.label(
                "地図で出来事を選ぶ → 上空へ → 現場へ → 撮影",
                x + 50.,
                y + h * 0.47,
                w - 100.,
                35.,
                Font::NormalCenter,
                MUTED,
            );
            self.button(
                "最初の一枚を撮りに行く",
                [x + w * 0.5 - 140., y + h * 0.60, 280., 42.],
                Act::Visit,
                true,
            );
        }
        let cw = (w - 72.) / 3.;
        let ch = (h - 140.) / 2.;
        for (k, i) in ids.iter().skip(self.page * 6).take(6).enumerate() {
            let p = self.journal.photos[*i].clone();
            let e = &EVENTS[p.event];
            let xx = x + 24. + (k % 3) as f32 * (cw + 12.);
            let yy = y + 90. + (k / 3) as f32 * (ch + 12.);
            self.gfx.stroke_rect(xx, yy, xx + cw, yy + ch, 1., LINE);
            self.gfx
                .fill_rect(xx, yy + ch - 57., xx + cw, yy + ch, WHITE);
            self.label(
                &format!("{} · {}", year_text(e.year), e.place),
                xx + 10.,
                yy + ch - 51.,
                cw - 20.,
                19.,
                Font::Small,
                MUTED,
            );
            self.label(
                e.title,
                xx + 10.,
                yy + ch - 29.,
                cw - 70.,
                26.,
                Font::Normal,
                INK,
            );
            self.label(
                &"★".repeat(p.quality as usize),
                xx + cw - 67.,
                yy + ch - 29.,
                60.,
                23.,
                Font::Small,
                GOLD,
            );
            self.buttons.push(Button {
                rect: [xx, yy, cw, ch],
                act: Act::Photo(*i),
            });
        }
        let pages = self.album_pages();
        self.label(
            &format!("{} / {}", self.page + 1, pages),
            x + w - 210.,
            y + 29.,
            70.,
            25.,
            Font::Normal,
            MUTED,
        );
        if self.page > 0 {
            self.button("‹", [x + w - 120., y + 23., 38., 34.], Act::Page(-1), false);
        }
        if self.page + 1 < pages {
            self.button("›", [x + w - 70., y + 23., 38., 34.], Act::Page(1), false);
        }
    }
    fn preview_ui(&mut self) {
        let i = self.preview.unwrap();
        let p = self.journal.photos[i].clone();
        let e = &EVENTS[p.event];
        let [x, y, w, h] = self.area();
        self.gfx.fill_rect(x, y, x + w, y + 59., PAPER);
        self.label(
            &format!("{}  /  {}  /  {}", year_text(e.year), e.place, e.title),
            x + 24.,
            y + 16.,
            w - 200.,
            34.,
            Font::Big,
            GREEN,
        );
        self.button(
            "写真帳に戻る",
            [x + w - 164., y + 15., 140., 34.],
            Act::Close,
            false,
        );
        self.gfx.fill_rect(x, y + h - 127., x + w, y + h, PAPER);
        self.label(
            e.body,
            x + 28.,
            y + h - 117.,
            w - 56.,
            60.,
            Font::Normal,
            INK,
        );
        self.label(
            &format!("参照: {}", e.source),
            x + 28.,
            y + h - 48.,
            w - 56.,
            40.,
            Font::Small,
            MUTED,
        );
    }
    fn help_ui(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        self.gfx.fill_rect(0., 0., w, h, [0.10, 0.18, 0.17, 0.65]);
        let x = w * 0.5 - 380.;
        let y = h * 0.5 - 260.;
        self.gfx.fill_round(x, y, x + 760., y + 520., 16., PAPER);
        self.label(
            "地図の向こうに、歴史の現場へ。",
            x + 32.,
            y + 25.,
            696.,
            48.,
            Font::TitleMid,
            GREEN,
        );
        let body="1  地域を選ぶ。中華・ヨーロッパ・日本などの年表を開きます。\n\n2  出来事や地図のピンを選ぶ。下の時間軸では好きな年代へ。\n\n3  「上空へ」で街へ。「現場に降りる」で街路の風景へ。\n\n4  左右ドラッグ・矢印で見回し、Space または撮影ボタン。\n\n5  写真帳で地域別・年代順に並ぶ写真を見返せます。\n\n地図はドラッグで移動、ホイール・＋−で拡大縮小。\nF2 写真帳 / F3 使い方 / F11 全画面 / Esc 戻る\n\n勢力の楕円は影響圏の概略です。国境ではありません。\n街は創作を含むデフォルメ復元。写真は構図を固定した記録です。";
        self.label(body, x + 38., y + 92., 684., 350., Font::Normal, INK);
        self.button(
            "探索に戻る",
            [x + 245., y + 457., 270., 40.],
            Act::Close,
            true,
        );
    }
}
fn inside(r: [f32; 4], x: f32, y: f32) -> bool {
    x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journal_roundtrip_and_corruption() {
        let mut j = Journal::new();
        j.photos.push(Photo {
            event: 4,
            heading: 15.,
            zoom: 1.2,
            time: 4.,
            quality: 3,
        });
        let b = j.bytes();
        let r = Journal::read(&b).unwrap();
        assert_eq!(r.photos, j.photos);
        assert_eq!(r.year, -221);
        for n in 0..b.len() {
            assert!(Journal::read(&b[..n]).is_err());
        }
        let mut bad = b.clone();
        bad[20] ^= 1;
        assert!(Journal::read(&bad).is_err());
    }
    #[test]
    fn every_event_can_be_visited_photographed_and_restored() {
        let mut g = Game::new(Gfx::new(1600, 900));
        for i in 0..EVENTS.len() {
            g.act(Act::Select(i));
            g.act(Act::Visit);
            assert_eq!(g.mode, Mode::Aerial);
            g.act(Act::Street);
            g.act(Act::Capture);
            assert!(g.found(i));
        }
        assert_eq!(g.journal.photos.len(), EVENTS.len());
        let b = g.journal.bytes();
        g.journal = Journal::new();
        g.apply_loaded(&b);
        assert_eq!(g.journal.photos.len(), EVENTS.len());
        for r in 1..REGIONS.len() {
            g.act(Act::Region(r));
            assert!(g
                .album_events()
                .iter()
                .all(|i| EVENTS[g.journal.photos[*i].event].region == r));
        }
    }
    #[test]
    fn camera_requires_scene_and_composition() {
        let mut g = Game::new(Gfx::new(1600, 900));
        g.capture();
        assert!(g.journal.photos.is_empty());
        g.act(Act::Visit);
        g.act(Act::Street);
        g.heading = 100.;
        g.capture();
        assert!(g.journal.photos.is_empty());
        g.heading = 0.;
        g.capture();
        assert_eq!(g.journal.photos[0].quality, 3);
        g.heading = 60.;
        g.capture();
        assert_eq!(g.journal.photos.len(), 1);
        assert_eq!(g.journal.photos[0].quality, 3);
    }
    #[test]
    fn all_views_render_and_search_input_works() {
        for (w, h, c) in [(1600, 900, false), (900, 420, true)] {
            let mut g = Game::new(Gfx::new(w, h));
            g.gfx.set_compact(c);
            for r in 0..REGIONS.len() {
                g.act(Act::Region(r));
                for m in [Mode::Map, Mode::Aerial, Mode::Street, Mode::Album] {
                    g.mode = m;
                    g.frame(&[], 0.016, true);
                }
                g.mode = Mode::Street;
                g.capture();
                g.mode = Mode::Album;
                g.render();
                g.preview = Some(0);
                g.render();
                g.preview = None;
            }
            g.help = true;
            g.render();
            g.searching = true;
            g.handle(Ev::Char('秦'));
            g.journal.region = 1;
            assert_eq!(g.filtered().len(), 1);
            g.handle(Ev::Key(8, true));
            assert!(g.query.is_empty());
        }
    }
}
