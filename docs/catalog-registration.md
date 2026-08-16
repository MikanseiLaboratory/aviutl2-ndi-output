# AviUtl2 Catalog 登録

最初の安定版 GitHub Release と `.au2pkg.zip` を公開し、クリーン環境でインストール・更新・アンインストールを確認してから提出します。

公開前ゲート:

1. 現行標準 NDI SDK の契約、version、公開日/取得日、SHA-256、30 日以内要件を `docs/ndi-sdk-record.md` に記録する
2. クリーン Windows x64 へ `.au2pkg.zip` のみを入れて load / 送受信 / 更新 / アンインストールを確認する

カタログ上のパッケージ名は「NDIライブ送出」です。プラグイン本体の表示名は「AviUtl2 Network Video Output」のままにし、NDI® は機能説明内で商標表記と https://ndi.video/ を添えて使用します。repository はこれらのゲートが満たされるまで private を維持します。

公開中のカタログアプリ（v0.3.3）の「JSON入力」は、[template.json](https://github.com/Neosku/aviutl2-catalog-data/blob/main/template.json) と同じ **配列** です。先頭は `[`、中身はパッケージ1件のオブジェクトです。単体の `{ ... }` を貼ると「入力形式が不正です」になります。貼るファイルは `catalog/package.json` です。

## 入力値

| 項目 | 値 |
| --- | --- |
| ID | `MikanseiLaboratory.NDIPlugin` |
| パッケージ名 | NDIライブ送出 |
| 種類 | 出力プラグイン |
| 作者 | 未完成成果物研究所 |
| ライセンス | MIT |
| リポジトリ | https://github.com/MikanseiLaboratory/aviutl2-network-video-output |
| 概要 (35 文字以内) | AviUtl2にNDI®の映像送出機能を追加します。 |
| 詳細 | `https://raw.githubusercontent.com/MikanseiLaboratory/aviutl2-network-video-output/refs/heads/main/README.md` |
| サムネイル | `catalog/image/MikanseiLaboratory.NDIPlugin_thumbnail.png`（登録画面で添付。JSON の images は空） |

貼り付け後に登録画面で行うこと:

1. サムネイル画像を添付する
2. バージョン欄で `aviutl2_ndi_live_output.aux2` を選び、XXH3-128 を計算する（JSON の `0000…` は仮値）
3. GitHub Release の `.au2pkg.zip` を公開してからインストーラーテストする

インストーラー source は GitHub Releases の owner `MikanseiLaboratory`、repo `aviutl2-network-video-output`、asset pattern `^aviutl2-network-video-output-v.*\.au2pkg\.zip$` です。zip ルートは `package.ini` + `Plugin/` + `Language/` です。

- install: download → extract → `{tmp}/Plugin` を `{pluginsDir}` へ copy、`{tmp}/Language` を `{dataDir}/Language` へ copy（フォルダ指定は中身をコピー。NDI runtime DLL も含む）
- uninstall: 配置した `.aux2`、同梱 NDI runtime DLL、付属データ、言語ファイルだけを delete
- バージョン検出対象: `{pluginsDir}/aviutl2_ndi_live_output.aux2`
