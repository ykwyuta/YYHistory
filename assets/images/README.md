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
