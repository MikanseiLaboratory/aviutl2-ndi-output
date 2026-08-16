# AviUtl2 Catalog 登録

最初の安定版 GitHub Release と `.au2pkg.zip` を公開し、クリーン環境でインストール・更新・アンインストールを確認してから提出します。

公開前ゲート:

1. 現行標準 NDI SDK の契約、version、公開日/取得日、SHA-256、30 日以内要件を `docs/ndi-sdk-record.md` に記録する
2. クリーン Windows x64 へ `.au2pkg.zip` のみを入れて load / 送受信 / 更新 / アンインストールを確認する

表示名は「AviUtl2 Network Video Output」です。NDI® は機能説明内だけで、商標表記と https://ndi.video/ を添えて使用します。repository はこれらのゲートが満たされるまで private を維持します。

カタログアプリの「パッケージ登録」画面から入力し、生成された変更を [`Neosku/aviutl2-catalog-data`](https://github.com/Neosku/aviutl2-catalog-data) への審査用 PR として提出してください。本リポジトリの `catalog/` はその入力原案です。

## 入力値

| 項目 | 値 |
| --- | --- |
| ID | `MikanseiLaboratory.aviutl2-ndi-output` |
| 種類 | 汎用プラグイン |
| 作者 | 未完成成果物研究所 |
| ライセンス | MIT |
| リポジトリ | https://github.com/MikanseiLaboratory/aviutl2-ndi-output |
| 概要 (35 文字以内) | 現在シーンをCUEでNDI®送出するプラグイン |
| 詳細 | `catalog/md/MikanseiLaboratory.aviutl2-ndi-output.md` |
| サムネイル | `catalog/image/MikanseiLaboratory.aviutl2-ndi-output_thumbnail.png` (1:1) |

インストーラー source は GitHub Releases の owner `MikanseiLaboratory`、repo `aviutl2-ndi-output`、asset pattern `^aviutl2-ndi-output-v.*\.au2pkg\.zip$` に固定します。

- install: download → extract → `Plugin` から `{pluginsDir}` へ copy、`Language` から `{appDir}/Language` へ copy
- uninstall: 配置した `.aux2`、同梱 NDI runtime DLL、付属データ、言語ファイルだけを delete
- バージョン検出対象: `Plugin/aviutl2_ndi_live_output.aux2`
- リリース時の XXH3-128 を `version[].file[].XXH3_128` に登録する

GitHub Releases 利用時、カタログ側は約 30 分間隔で更新確認します。release asset 名は以後互換に保ち、破壊的な配置変更時はカタログ定義も同時更新します。

PR 前にカタログのテスト機能で、ダウンロード、展開先、ハッシュ、更新検出、アンインストール、ライセンス全文表示を検証してください。
