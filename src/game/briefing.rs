//! 時代の説明：転移の前に、クロノがこれから行く時代の前後の流れをやさしく説明する。
//!
//! はじめて行く時代では「転移する」を押すと自動で始まり、読み終えると転移する。
//! 時代の札と地図の右の欄の「クロノの説明」から、いつでも読み直せる。
//! 文は小学生にもわかる言葉で漢字を使って書く（読み上げで補う）。不確かなことは「〜といわれる」と書く。
use super::*;

pub struct BriefPage {
    pub title: &'static str,
    pub body: &'static str,
}

const fn page(title: &'static str, body: &'static str) -> BriefPage {
    BriefPage { title, body }
}

/// 時代ごとの説明（ERAS と同じ並び）
pub static BRIEFINGS: [&[BriefPage]; 7] = [
    // ---------------------------------------------------------------- 共和政ローマ
    &[
        page("どんな場所？", "ローマは、今のイタリアの真ん中にある町。\nティベリス川のほとりの7つの丘に広がっていたよ。"),
        page("それまでに何があった？", "ローマには昔、王さまがいたけれど、紀元前509年ごろ（と伝えられる）に追い出して、「共和政」になった。\n選ばれた人たちと元老院が国を動かし、戦いで地中海のまわりに広がっていった。\nでも、力のある将軍どうしの争いが激しくなったんだ。"),
        page("今、何が起きている？", "カエサルは、ガリア（今のフランスあたり）を征服した将軍。\n紀元前49年にルビコン川を渡って内戦を始め、ポンペイウスたちに勝った。\n今は「終身独裁官」として、とても強い力を持っている。それをよく思わない議員たちもいるよ。"),
        page("このあとどうなる？", "カエサルが殺されたあと、また内戦が起こる。\n最後に勝ったオクタウィアヌスは、紀元前27年に「アウグストゥス」の名を贈られ、ローマは皇帝が治める国へ変わっていくんだ。"),
        page("見どころ", "人びとの話を聞いて、カエサルがなぜねらわれたのか、考えてみよう。\n3月15日の出来事が起きた場所にも行ってみてね。"),
    ],
    // ---------------------------------------------------------------- 唐の都・長安
    &[
        page("どんな場所？", "長安は、今の中国の西安。\n「唐」という国の都で、100万人近くが暮らしたともいわれる、世界でもとても大きな町だったよ。"),
        page("それまでに何があった？", "唐は618年に始まった国。\nシルクロードで西の国ぐにとつながり、日本からも「遣唐使」が学びに来た。\nいろいろな国の人や物が集まる町になったんだ。"),
        page("今、何が起きている？", "玄宗という皇帝の時代。\n前の半分はよく治まって「開元の治」と呼ばれた。\n742年に年号が「天宝」に変わったころから、皇帝は楊貴妃を大切にし、政治をほかの人に任せることが増えていく。"),
        page("このあとどうなる？", "755年、安禄山が反乱を起こす（安史の乱）。\n乱は763年まで続き、唐の力は大きく弱まった。\n唐は907年に滅びるよ。"),
        page("見どころ", "詩人の李白や、日本から来た阿倍仲麻呂に会ってみよう。\n皇帝のまわりで何が起きていたのかも調べてね。"),
    ],
    // ---------------------------------------------------------------- 平安京
    &[
        page("どんな場所？", "平安京は、今の京都市。\n794年に都になった、碁盤の目のように道が並ぶ町だよ。"),
        page("それまでに何があった？", "藤原氏という貴族の家は、娘を天皇のきさきにした。\n生まれた子どもが天皇になると、そのおじいさんとして「摂政」や「関白」になり、政治を動かした。\nこれを「摂関政治」というよ。"),
        page("今、何が起きている？", "藤原道長の時代。\n道長は娘たちを次々にきさきにして、とても強い力を持った。\nきさきに仕えた女性たちが、かな文字で『源氏物語』や『枕草子』を書いたんだ。"),
        page("このあとどうなる？", "道長の子どもの頼通も、長く政治を動かした。\nやがて1086年に白河上皇が「院政」を始め、そのあと武士の力が強くなっていくよ。"),
        page("見どころ", "紫式部や清少納言、道長に会ってみよう。\nだれが、だれの娘をきさきにしたのか、注目してね。"),
    ],
    // ---------------------------------------------------------------- ルネサンスのフィレンツェ
    &[
        page("どんな場所？", "フィレンツェは、イタリアの真ん中より少し北にある町。\nアルノ川が流れ、銀行や毛織物の商売で豊かになったよ。"),
        page("それまでに何があった？", "1400年代にはメディチ家が力を持ち、芸術家や学者を助けた。\n昔のギリシャやローマの学問や芸術を見直す「ルネサンス」が栄えた。\nでも1494年、メディチ家は町から追い出されたんだ。"),
        page("今、何が起きている？", "フィレンツェは「共和国」として、選ばれた人たちが治めている。\n若いミケランジェロが大きなダヴィデ像を彫り、レオナルド・ダ・ヴィンチも町に戻ってきた。\nマキャヴェッリは、共和国の仕事であちこちへ出かけているよ。"),
        page("このあとどうなる？", "1512年、メディチ家が町に戻ってくる。\nミケランジェロはローマでシスティーナ礼拝堂の天井画を描き、マキャヴェッリは『君主論』を書いたよ。"),
        page("見どころ", "ダヴィデ像がどこに置かれるのか、最後まで見届けよう。\nレオナルドとミケランジェロ、二人の天才に会ってみてね。"),
    ],
    // ---------------------------------------------------------------- テノチティトラン
    &[
        page("どんな場所？", "テノチティトランは、テスココ湖という湖の島に造られた、メシカ（アステカ）の都。\n今のメキシコシティのところだよ。"),
        page("それまでに何があった？", "メシカの人びとは1325年ごろに、この島に町を造ったと伝えられる。\n近くの町と手を結び、多くの町から貢ぎ物を受け取る強い国になったんだ。"),
        page("今、何が起きている？", "1492年にコロンブスがカリブ海の島に着いてから、スペインの人びとがアメリカに来るようになった。\n1519年、コルテスたちがこの都にやって来る。\nメシカと対立していたトラスカラの人びとも、スペイン側についたよ。"),
        page("このあとどうなる？", "1521年、都は落ちる。\nスペインはこの地を支配し、テノチティトランのあとにメキシコシティを造った。\n天然痘などの病気でも、とても多くの人が亡くなったんだ。"),
        page("見どころ", "コルテス、モクテスマ、通訳のマリンツィンの話を比べてみよう。\nどちらの側から見るかで、同じ出来事の見え方が変わるよ。"),
    ],
    // ---------------------------------------------------------------- 革命のパリ
    &[
        page("どんな場所？", "パリはフランスの都。\n王さまの住まい（ヴェルサイユ宮殿）は、パリから少し離れたヴェルサイユにあったよ。"),
        page("それまでに何があった？", "フランスでは、王さまがとても強い力を持ち、聖職者と貴族には特別な権利があった。\n国の人のほとんどは平民で、重い税を払っていた。\nアメリカの独立戦争を助けたことなどで、国の借金はとてもふくらんでいたんだ。"),
        page("今、何が起きている？", "1789年、お金の問題を話し合うために「全国三部会」が開かれる。\n平民の代表は「国民議会」をつくり、パリの人びとも立ち上がる。\nこれがフランス革命の始まりだよ。"),
        page("このあとどうなる？", "1792年に王さまの政治は終わり、1793年にルイ16世は処刑される。\n多くの人が殺される恐ろしい時代もあった。\nやがて軍人のナポレオンが力を持つようになるよ。"),
        page("見どころ", "平民の代表たちが何を誓い、何を決めたのか、調べてみよう。\nヴェルサイユとパリを行ったり来たりしてね。"),
    ],
    // ---------------------------------------------------------------- 幕末の江戸
    &[
        page("どんな場所？", "江戸は、今の東京。\n徳川の将軍の城（江戸城）を真ん中に、たくさんの人が暮らす大きな町だったよ。"),
        page("それまでに何があった？", "1603年から徳川家が「江戸幕府」を開き、260年あまり日本を治めた。\n1853年にペリーの船が来て、幕府はほかの国とつき合うことになった。\n幕府を倒そうとする薩摩藩や長州藩が、力をつけていったんだ。"),
        page("今、何が起きている？", "1867年、将軍の徳川慶喜は、政治をする力を天皇に返した（大政奉還）。\n1868年、京都の近くで戦いが始まり（鳥羽・伏見の戦い）、新政府の軍隊が江戸に迫っているよ。"),
        page("このあとどうなる？", "江戸城は戦わずに明け渡され、江戸は「東京」と呼ばれるようになる。\n年号が「明治」に変わり、新しい国づくりが始まるよ。"),
        page("見どころ", "勝海舟と西郷隆盛の話し合いに注目しよう。\n江戸の町がどうして戦争にならずに済んだのか、考えてみてね。"),
    ],
];

/// 1ページ目の前に、クロノがかける言葉
pub const OPENING: &str = "これから行く時代について、説明するね！";

pub fn pages(era: usize) -> &'static [BriefPage] {
    BRIEFINGS[era]
}

pub fn speech(era: usize, page: usize) -> String {
    let p = &pages(era)[page];
    let head = if page == 0 {
        format!("{OPENING}{}。{}。", ERAS[era].name, ERAS[era].years)
    } else {
        String::new()
    };
    format!("{head}{}。{}", p.title, p.body.replace('\n', ""))
}

impl Game {
    /// 時代の説明を開く（then: 読み終えたら転移する先）
    pub(super) fn open_briefing(&mut self, era: usize, then: Option<(usize, Option<usize>)>) {
        self.briefing_then = then;
        self.show_briefing(era, 0);
    }

    pub(super) fn show_briefing(&mut self, era: usize, page: usize) {
        let page = page.min(pages(era).len() - 1);
        self.overlay = Overlay::Briefing(era, page);
        self.story_t = 0.;
        self.say(speech(era, page));
    }

    /// 説明を終える（転移の前なら、そのまま転移する）
    pub(super) fn finish_briefing(&mut self) {
        self.overlay = Overlay::None;
        self.hush();
        if let Some((era, place)) = self.briefing_then.take() {
            self.start_shift(era, place);
        }
    }

    pub(super) fn briefing_ui(&mut self, era: usize, page: usize) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let compact = self.gfx.compact;
        let fade = (self.story_t * 3.).min(1.);
        self.starfield(0.97);
        // 時代の扉絵をうすく
        if let Some(img) = self.gfx.image(&images::era_art(era)) {
            cover(&mut self.gfx, &img, [0., 0., w, h], 16. / 9., 0.28);
        }
        let all = pages(era);
        let p = &all[page];
        let cw = (w - 40.).min(940.);
        let ch = (h - self.top() - if compact { 28. } else { 60. }).min(if compact {
            330.
        } else {
            430.
        });
        let (x, y) = ((w - cw) / 2., self.top() + if compact { 10. } else { 40. });
        self.gfx
            .fill_round(x, y, x + cw, y + ch, 16., [0.05, 0.09, 0.17, 0.94]);
        self.gfx
            .stroke_round(x, y, x + cw, y + ch, 16., 1.5, alpha(CYAN, 0.8));
        // クロノ
        let cs = if compact { 26. } else { 40. };
        let (cx, cy) = (x + 20. + cs * 1.4, y + 20. + cs * 1.6);
        self.gfx
            .fill_circle(cx, cy, cs + 16., [0.02, 0.04, 0.10, 0.6]);
        self.chrono(cx, cy, cs, page == 0);
        let tx = x + 40. + cs * 2.8;
        let tw = cw - (tx - x) - 24.;
        let opening = if page == 0 {
            OPENING
        } else {
            "クロノの時代説明"
        };
        self.text(opening, tx, y + 14., tw, 22., Font::Normal, CYAN);
        self.text(
            &format!("{}　{}", ERAS[era].years, ERAS[era].name),
            tx,
            y + 38.,
            tw,
            20.,
            Font::Small,
            GOLD,
        );
        self.text(
            p.title,
            tx,
            y + 60.,
            tw,
            if compact { 30. } else { 40. },
            Font::Big,
            GOLD,
        );
        let by = y + if compact { 96. } else { 112. };
        let bh = ch - (by - y) - 56.;
        self.scroll_areas.push(([x, by, cw, bh], Scroll::Overlay));
        let off = self.scroll[Scroll::Overlay as usize];
        let body_w = cw - 60.;
        let th = self.gfx.measure_h(p.body, Font::BigCenter, body_w);
        self.gfx.clip_begin(x, by, x + cw, by + bh);
        self.text(
            p.body,
            x + 30.,
            by + 4. - off,
            body_w,
            th.max(bh - 8.),
            Font::BigCenter,
            alpha(TEXT, fade),
        );
        self.gfx.clip_end();
        self.scroll_max[Scroll::Overlay as usize] = th + 8. - bh;
        // ページの印とボタン
        for k in 0..all.len() {
            let dx = x + cw / 2. + (k as f32 - (all.len() - 1) as f32 / 2.) * 18.;
            self.gfx.fill_circle(
                dx,
                y + ch - 24.,
                4.,
                if k == page { GOLD } else { alpha(MUTED, 0.5) },
            );
        }
        let last = page + 1 == all.len();
        if page > 0 {
            self.button(
                "◀ もどる",
                [x + 16., y + ch - 44., 120., 34.],
                Act::BriefBack,
                false,
            );
        }
        let label = match (last, self.briefing_then.is_some()) {
            (true, true) => "出発！",
            (true, false) => "わかった！",
            _ => "つぎへ ▶",
        };
        self.button(
            label,
            [x + cw - 176., y + ch - 46., 160., 38.],
            Act::Next,
            true,
        );
        self.button(
            if self.briefing_then.is_some() {
                "とばして出発"
            } else {
                "とじる"
            },
            [
                w - if compact { 200. } else { 220. },
                h - 44.,
                if compact { 184. } else { 200. },
                32.,
            ],
            Act::BriefSkip,
            false,
        );
        self.speaker_button(x + cw - 96., y + 12.);
    }
}
