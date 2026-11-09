# assets/images

ゲームで使う画像を置く場所です。必要な画像と、それぞれの大きさ・描く内容・時代考証の注意は [docs/images.md](../../docs/images.md) にあります。

```
assets/images/
  portraits/<人物ID>.jpg   人物の肖像 512×512
  events/<出来事ID>.jpg    出来事の場面 1280×720
  treasures/<時代ID>.png   秘宝（透過PNG）512×512
  eras/<時代ID>.jpg        時代の扉絵 1600×900
  maps/<時代ID>.jpg        時代の地図 1600×1000
  ui/akashic.jpg           試練の背景 1600×900
```

置いたら `npm run images:check` で、抜け・大きさ違い・パスの間違いを確かめてください。`manifest.json` は `src/images.rs` から生成したものなので、手で編集しないでください。

## 生成画像

組み込みの `image_gen` で制作し、ImageMagick で規定の寸法・形式へ変換しています。秘宝は生成時の透過を維持します。記録が残っている109枚の全文プロンプトと、全110枚の生成元ファイル名・SHA-256を [generation-prompts.json](generation-prompts.json) に保存しています。ローマの扉絵（`eras/rome.jpg`）はプロンプトが未記録（`null`）です。

人物・建築・出来事・品物はいずれも教育用の復元イメージです。写真・史料原本・実物の正確な複製ではありません。地図はゲームが重ねる地点の座標に合わせた概略図です。画像の細部や銘文を史料の代わりに使わず、本文の説明と出典を参照してください。
