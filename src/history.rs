//! Dates use historical numbering (negative = BCE, no year zero).
//! Coordinates and influence areas are intentionally schematic.
pub struct Region {
    pub name: &'static str,
    pub lon: f32,
    pub lat: f32,
    pub span: f32,
}
pub static REGIONS: &[Region] = &[
    Region {
        name: "世界",
        lon: 15.,
        lat: 0.,
        span: 360.,
    },
    Region {
        name: "中華・東アジア",
        lon: 108.,
        lat: 33.,
        span: 64.,
    },
    Region {
        name: "ヨーロッパ",
        lon: 14.,
        lat: 48.,
        span: 62.,
    },
    Region {
        name: "日本",
        lon: 136.,
        lat: 36.,
        span: 26.,
    },
    Region {
        name: "西アジア・北アフリカ",
        lon: 35.,
        lat: 28.,
        span: 74.,
    },
    Region {
        name: "南・東南アジア",
        lon: 78.,
        lat: 23.,
        span: 44.,
    },
    Region {
        name: "南北アメリカ",
        lon: -90.,
        lat: 15.,
        span: 130.,
    },
    Region {
        name: "アフリカ・オセアニア",
        lon: 43.,
        lat: -12.,
        span: 220.,
    },
];
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Chinese,
    European,
    Japanese,
    Desert,
    Indian,
    American,
    African,
    Oceanic,
}
pub struct Event {
    /// Stable identifier; saves do not depend on catalog ordering.
    pub id: &'static str,
    pub year: i32,
    pub region: usize,
    pub title: &'static str,
    pub place: &'static str,
    pub lon: f32,
    pub lat: f32,
    pub style: Style,
    pub category: &'static str,
    pub body: &'static str,
    pub source: &'static str,
}
macro_rules! event {
    ($id:literal,$year:expr,$region:expr,$title:literal,$place:literal,$lon:expr,$lat:expr,$style:ident,$cat:literal,$body:literal,$source:literal) => {
        Event {
            id: $id,
            year: $year,
            region: $region,
            title: $title,
            place: $place,
            lon: $lon,
            lat: $lat,
            style: Style::$style,
            category: $cat,
            body: $body,
            source: $source,
        }
    };
}
pub static EVENTS: &[Event] = &[
    event!("giza",-2560,4,"ギザの大ピラミッド","ギザ",31.13,29.98,Desert,"建築","古王国時代、クフ王の巨大なピラミッドが築かれた。石材の運搬、働く人々、ナイルの流れを観察しよう。年代はおよそのもの。","https://www.si.edu/spotlight/ancient-egypt/pyramid"),
    event!("athens",-432,2,"パルテノンの時代","アテネ",23.73,37.97,European,"文化","アテネのアクロポリスに神殿が姿を現した。民主政の市民から女性・奴隷・在留外国人が除かれていたことも、当時の社会を考える手がかりになる。","https://www.theacropolismuseum.gr/en/parthenon"),
    event!("alexandria",-331,4,"アレクサンドリアの建設","アレクサンドリア",29.92,31.20,Desert,"都市","アレクサンドロスによって地中海沿いに都市が建設された。やがて交易と学問が集まる場所になる。港と街のつながりを探そう。","https://www.metmuseum.org/toah/hd/alex/hd_alex.htm"),
    event!("ashoka",-250,5,"アショーカ王の石柱","サールナート",83.02,25.38,Indian,"思想","マウリヤ朝の王は、統治や人々の暮らしに関する言葉を石に刻ませた。石柱を通して、文字が広い地域に考えを伝える働きを見る。年代は概数。","https://www.metmuseum.org/toah/ht/04/ssa.html"),
    event!("qin",-221,1,"秦の中国統一","咸陽",108.71,34.33,Chinese,"政治","秦が諸国を征服し、始皇帝のもとで統一国家を築いた。文字・貨幣・度量衡の統一と、厳しい支配の両方に目を向けよう。","https://www.britishmuseum.org/collection/galleries/china-and-south-asia"),
    event!("silkroad",-138,1,"張騫、西域へ","長安",108.94,34.34,Chinese,"交流","漢の使者、張騫が西へ出発した。後のシルクロードにつながる交流は、一人や一本の道だけでなく、多くの人と道の積み重ねだった。","https://www.metmuseum.org/toah/hd/silk/hd_silk.htm"),
    event!("colosseum",80,2,"コロッセウムの完成","ローマ",12.49,41.89,European,"建築","ローマに巨大な円形闘技場が開かれた。建築の技術と大勢の観客、その裏にある暴力や奴隷制も歴史の一部である。","https://colosseo.it/en/area/the-colosseum/"),
    event!("paper",105,1,"紙づくりの改良","洛陽",112.45,34.62,Chinese,"技術","蔡倫による製紙法の改良が伝えられる年。紙自体はそれ以前にも存在した。植物の繊維から情報を伝える道具が生まれる過程を観察しよう。","https://www.metmuseum.org/toah/ht/05/eac.html"),
    event!("nara",710,3,"平城京への遷都","奈良",135.80,34.69,Japanese,"都市","都が平城京に移った。東アジアとの交流を受けた都の道と建物、そこで生活する人々に目を向けよう。","https://www.nabunken.go.jp/heijo/"),
    event!("tang",742,1,"唐の都、長安","長安",108.94,34.34,Chinese,"交流","唐の都には遠い地域から商人や留学生が集まった。市場や門を訪ね、品物、服装、建物から国際的な都市の暮らしを想像しよう。","https://www.metmuseum.org/toah/hd/tang/hd_tang.htm"),
    event!("baghdad",762,4,"バグダードの建設","バグダード",44.37,33.31,Desert,"都市","アッバース朝が新しい都を建設した。交易の道が交わり、多様な学問や翻訳が発展していく都市の始まりを観察する。","https://www.metmuseum.org/toah/hd/abba/hd_abba.htm"),
    event!("heian",794,3,"平安京への遷都","京都",135.75,35.01,Japanese,"都市","新しい都、平安京へ。碁盤の目のような通りと宮殿を見つけ、都の姿が人々の生活にどう関わったか考えてみよう。","https://www.kyohaku.go.jp/eng/"),
    event!("song",1088,1,"宋の科学と都市","開封",114.31,34.80,Chinese,"技術","沈括の『夢渓筆談』がまとめられた頃。印刷、羅針盤、都市の市場など、知識と商業が発展した宋の世界を訪れる。年代は概数。","https://www.metmuseum.org/toah/hd/nsong/hd_nsong.htm"),
    event!("angkor",1150,5,"アンコール・ワットの時代","アンコール",103.87,13.41,Indian,"建築","クメール王朝の大寺院が築かれた頃。水を蓄える施設と建物の関係を探そう。この項目は南・東南アジアの交流圏として収録。","https://whc.unesco.org/en/list/668/"),
    event!("mongol",1206,1,"モンゴル帝国の成立","モンゴル高原",108.0,47.0,Chinese,"政治","テムジンがチンギス・カンとなった。大陸規模の交流の拡大と、征服による破壊や人々の被害をあわせて考える。","https://www.metmuseum.org/toah/hd/yuan/hd_yuan.htm"),
    event!("mali",1324,7,"マンサ・ムーサの巡礼","カイロ",31.24,30.04,African,"交流","マリ王国の王マンサ・ムーサが巡礼の途中でカイロを訪れた。西アフリカの金とサハラ交易が、遠い都市を結んでいた。","https://www.metmuseum.org/toah/hd/mali/hd_mali.htm"),
    event!("tenochtitlan",1325,6,"湖上の都市の始まり","テノチティトラン",-99.13,19.43,American,"都市","メシカの人々による都市建設の伝承上の年。湖、堤道、耕作地が結びついた都市へ。年代には伝承と研究上の幅がある。","https://www.metmuseum.org/toah/hd/azte/hd_azte.htm"),
    event!("ming",1405,1,"鄭和の航海","南京",118.80,32.06,Chinese,"交流","明の鄭和が率いる船団が最初の航海へ出発した。港に並ぶ船と交易品から、インド洋につながる交流を追いかけよう。","https://www.metmuseum.org/toah/hd/ming/hd_ming.htm"),
    event!("printing",1450,2,"活版印刷が広がる","マインツ",8.27,50.00,European,"技術","グーテンベルクの印刷技術が形になった頃。東アジアの先行する印刷技術にも目を向けながら、本が広まるしくみを観察しよう。年代は概数。","https://www.gutenberg-museum.de/en/"),
    event!("florence",1504,2,"ルネサンスの広場","フィレンツェ",11.26,43.77,European,"文化","ミケランジェロのダヴィデ像が広場に置かれた。工房、建築、広場を通じて、芸術と都市の政治が結びついていた時代を見る。","https://www.galleriaaccademiafirenze.it/en/artworks/david-michelangelo/"),
    event!("taj",1648,5,"タージ・マハル","アーグラ",78.04,27.18,Indian,"建築","ムガル帝国の時代、白い大理石の霊廟の主要部分が完成した。複合施設の工事はその後も続いた。庭と建物の対称性を探してみよう。","https://whc.unesco.org/en/list/252/"),
    event!("edo",1603,3,"江戸幕府の始まり","江戸",139.75,35.68,Japanese,"政治","徳川家康が征夷大将軍となった。城、堀、道が都市を形づくり、江戸は大きな町へ成長していく。","https://www.edo-tokyo-museum.or.jp/en/"),
    event!("london",1666,2,"ロンドン大火と再建","ロンドン",-0.09,51.51,European,"災害","大火がロンドンの広い範囲を焼いた。被害を記憶し、その後の街路や建築の再建が都市をどう変えたかを観察する。","https://www.londonmuseum.org.uk/collections/london-stories/great-fire-london/"),
    event!("independence",1776,6,"独立宣言の採択","フィラデルフィア",-75.15,39.95,American,"政治","7月4日、大陸会議が独立宣言を採択した。平等の理念と、当時なお続いていた奴隷制や先住民の排除との隔たりにも目を向けよう。","https://www.archives.gov/founding-docs/declaration"),
    event!("revolution",1789,2,"球戯場の誓い","ヴェルサイユ",2.12,48.80,European,"政治","6月20日、議員たちは憲法を定めるまで解散しないと誓った。フランス革命の一場面。人々が集まった場所から政治の変化を記録しよう。","https://en.chateauversailles.fr/discover/history/key-dates/jeu-paume-oath-1789"),
    event!("railway",1830,2,"都市を結ぶ鉄道","リヴァプール",-2.98,53.41,European,"技術","リヴァプールとマンチェスターを結ぶ鉄道が開業した。人や物の移動が変わる一方、工場の労働や煙による問題も増えていった。","https://www.scienceandindustrymuseum.org.uk/"),
    event!("meiji",1868,3,"明治の始まり","東京",139.75,35.68,Japanese,"政治","政権の移行とともに江戸は東京となり、元号は明治へ。制度や生活が一度に全て変わったわけではない。街の新旧の姿を観察する。","https://www.ndl.go.jp/modern/"),
    event!("wright",1903,6,"ライト兄弟の動力飛行","キティホーク",-75.67,36.02,American,"技術","12月17日、ライト兄弟が動力飛行機による飛行を行った。機体だけでなく、試験を支えた風、砂地、道具にも目を向けよう。","https://airandspace.si.edu/explore/stories/1903-wright-flyer"),
    event!("salt",1930,5,"塩の行進","ダンディー",72.80,20.89,Indian,"政治","ガンディーらが植民地の塩税に抗議して歩いた。非暴力の抵抗に加わった人々と、暮らしに身近な塩が持つ意味を考える。","https://www.gandhiheritageportal.org/"),
    event!("apollo",1969,6,"月へ向かった人々","ヒューストン",-95.09,29.55,American,"技術","7月20日、アポロ11号が月に着陸した。ここは地球の管制拠点。月面そのものではなく、地上で計画を支えた人々の場所を撮影する。","https://www.nasa.gov/mission/apollo-11/"),
    event!("sydney",1973,7,"シドニー・オペラハウス","シドニー",151.22,-33.86,Oceanic,"建築","帆のような屋根を持つオペラハウスが開場した。この場所の先住民の歴史と、建設を支えた技術や人々にも目を向けよう。","https://www.sydneyoperahouse.com/our-story"),
    event!("berlin",1989,2,"ベルリンの壁が開く","ベルリン",13.38,52.52,European,"政治","11月9日、東西を隔てていた壁の国境通過が開かれた。統一は翌年。壁の前に集まる人々から、この転換点を記録する。","https://www.stiftung-berliner-mauer.de/en/"),
];

pub fn year_text(year: i32) -> String {
    if year < 0 {
        format!("紀元前{}年", -year)
    } else {
        format!("{}年", year)
    }
}
pub fn matching(region: usize, query: &str) -> Vec<usize> {
    EVENTS
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            (region == 0 || e.region == region)
                && (query.is_empty()
                    || format!(
                        "{} {} {} {} {}",
                        e.title, e.place, e.category, e.year, e.body
                    )
                    .to_lowercase()
                    .contains(&query.to_lowercase()))
        })
        .map(|(i, _)| i)
        .collect()
}
pub struct Power {
    pub name: &'static str,
    pub from: i32,
    pub to: i32,
    pub lon: f32,
    pub lat: f32,
    pub rx: f32,
    pub ry: f32,
    pub color: [f32; 4],
}
pub static POWERS: &[Power] = &[
    Power {
        name: "周・諸侯国",
        from: -1046,
        to: -256,
        lon: 111.,
        lat: 35.,
        rx: 14.,
        ry: 9.,
        color: [0.62, 0.62, 0.37, 0.25],
    },
    Power {
        name: "古代ギリシアの諸ポリス",
        from: -800,
        to: -146,
        lon: 24.,
        lat: 38.,
        rx: 7.,
        ry: 5.,
        color: [0.36, 0.6, 0.68, 0.25],
    },
    Power {
        name: "マウリヤ朝",
        from: -322,
        to: -185,
        lon: 80.,
        lat: 25.,
        rx: 13.,
        ry: 11.,
        color: [0.76, 0.57, 0.31, 0.25],
    },
    Power {
        name: "東ローマ帝国",
        from: 395,
        to: 1453,
        lon: 27.,
        lat: 39.,
        rx: 10.,
        ry: 7.,
        color: [0.63, 0.47, 0.73, 0.25],
    },
    Power {
        name: "フランス王国",
        from: 987,
        to: 1792,
        lon: 2.,
        lat: 47.,
        rx: 6.,
        ry: 6.,
        color: [0.43, 0.57, 0.76, 0.25],
    },
    Power {
        name: "マリ王国",
        from: 1235,
        to: 1600,
        lon: -5.,
        lat: 15.,
        rx: 10.,
        ry: 7.,
        color: [0.75, 0.57, 0.28, 0.25],
    },
    Power {
        name: "アステカの都市連合",
        from: 1428,
        to: 1521,
        lon: -99.,
        lat: 19.,
        rx: 6.,
        ry: 5.,
        color: [0.46, 0.66, 0.49, 0.25],
    },
    Power {
        name: "インカ帝国",
        from: 1438,
        to: 1533,
        lon: -74.,
        lat: -13.,
        rx: 6.,
        ry: 17.,
        color: [0.75, 0.58, 0.34, 0.25],
    },
    Power {
        name: "イギリス",
        from: 1707,
        to: 2000,
        lon: -2.,
        lat: 54.,
        rx: 5.,
        ry: 7.,
        color: [0.51, 0.57, 0.72, 0.25],
    },
    Power {
        name: "アメリカ合衆国",
        from: 1776,
        to: 2000,
        lon: -98.,
        lat: 39.,
        rx: 22.,
        ry: 12.,
        color: [0.37, 0.57, 0.75, 0.25],
    },
    Power {
        name: "日本",
        from: 1868,
        to: 2000,
        lon: 138.,
        lat: 37.,
        rx: 6.,
        ry: 9.,
        color: [0.76, 0.46, 0.45, 0.25],
    },
    Power {
        name: "ソビエト連邦",
        from: 1922,
        to: 1991,
        lon: 80.,
        lat: 58.,
        rx: 45.,
        ry: 14.,
        color: [0.75, 0.48, 0.43, 0.25],
    },
    Power {
        name: "インド",
        from: 1947,
        to: 2000,
        lon: 79.,
        lat: 23.,
        rx: 11.,
        ry: 14.,
        color: [0.76, 0.6, 0.34, 0.25],
    },
    Power {
        name: "中華人民共和国",
        from: 1949,
        to: 2000,
        lon: 105.,
        lat: 35.,
        rx: 24.,
        ry: 16.,
        color: [0.76, 0.48, 0.36, 0.25],
    },
    Power {
        name: "エジプト古王国",
        from: -2686,
        to: -2181,
        lon: 31.,
        lat: 27.,
        rx: 5.,
        ry: 7.,
        color: [0.82, 0.59, 0.22, 0.26],
    },
    Power {
        name: "アケメネス朝",
        from: -550,
        to: -330,
        lon: 45.,
        lat: 32.,
        rx: 22.,
        ry: 10.,
        color: [0.77, 0.54, 0.37, 0.27],
    },
    Power {
        name: "秦",
        from: -221,
        to: -206,
        lon: 110.,
        lat: 33.,
        rx: 12.,
        ry: 10.,
        color: [0.63, 0.40, 0.36, 0.3],
    },
    Power {
        name: "漢",
        from: -206,
        to: 220,
        lon: 109.,
        lat: 33.,
        rx: 17.,
        ry: 12.,
        color: [0.83, 0.44, 0.36, 0.3],
    },
    Power {
        name: "ローマ帝国",
        from: -27,
        to: 395,
        lon: 16.,
        lat: 39.,
        rx: 25.,
        ry: 10.,
        color: [0.63, 0.47, 0.73, 0.28],
    },
    Power {
        name: "唐",
        from: 618,
        to: 907,
        lon: 108.,
        lat: 36.,
        rx: 20.,
        ry: 13.,
        color: [0.85, 0.48, 0.38, 0.3],
    },
    Power {
        name: "アッバース朝",
        from: 750,
        to: 1258,
        lon: 44.,
        lat: 31.,
        rx: 18.,
        ry: 11.,
        color: [0.36, 0.61, 0.55, 0.3],
    },
    Power {
        name: "宋",
        from: 960,
        to: 1279,
        lon: 115.,
        lat: 30.,
        rx: 10.,
        ry: 8.,
        color: [0.81, 0.61, 0.31, 0.3],
    },
    Power {
        name: "モンゴル帝国・諸政権",
        from: 1206,
        to: 1368,
        lon: 83.,
        lat: 44.,
        rx: 42.,
        ry: 15.,
        color: [0.44, 0.60, 0.78, 0.3],
    },
    Power {
        name: "明",
        from: 1368,
        to: 1644,
        lon: 111.,
        lat: 33.,
        rx: 15.,
        ry: 13.,
        color: [0.85, 0.47, 0.34, 0.3],
    },
    Power {
        name: "オスマン帝国",
        from: 1299,
        to: 1922,
        lon: 31.,
        lat: 37.,
        rx: 13.,
        ry: 10.,
        color: [0.36, 0.64, 0.56, 0.3],
    },
    Power {
        name: "ムガル帝国",
        from: 1526,
        to: 1857,
        lon: 78.,
        lat: 25.,
        rx: 10.,
        ry: 12.,
        color: [0.77, 0.57, 0.36, 0.3],
    },
    Power {
        name: "江戸幕府",
        from: 1603,
        to: 1867,
        lon: 137.,
        lat: 36.,
        rx: 5.,
        ry: 8.,
        color: [0.58, 0.57, 0.78, 0.3],
    },
    Power {
        name: "清",
        from: 1644,
        to: 1912,
        lon: 105.,
        lat: 36.,
        rx: 24.,
        ry: 17.,
        color: [0.86, 0.64, 0.24, 0.3],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_is_valid_and_searchable() {
        let mut ids = std::collections::HashSet::new();
        for e in EVENTS {
            assert!(ids.insert(e.id));
            assert!(e.year != 0);
            assert!((1..REGIONS.len()).contains(&e.region));
            assert!(e.lon.abs() <= 180. && e.lat.abs() <= 90.);
            assert!(e.source.starts_with("https://"));
        }
        assert_eq!(matching(1, "秦").len(), 1);
        for region in 1..REGIONS.len() {
            assert!(!matching(region, "").is_empty());
        }
    }
}
