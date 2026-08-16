# AviUtl2 Network Video Output

AviUtl2 の今開いているシーンを描画して、[NDI®](https://ndi.video/) で送出する出力プラグインです。

NDI® is a registered trademark of Vizrt NDI AB.

## できること

- 今開いているシーンを描画し、先頭の映像を送出し始めます。
- **CUE** ボタンで送出・再生を開始します。AviUtl2 の再生ボタンは使いません。
- 再生が遅れたときは途中を飛ばして、いまの位置から送ります。再生後は最後の映像のまま送出します。
- 映像とステレオ音声を送信します。アルファの送信は設定で切り替えられます。

## 動作条件

- Windows x64
- AviUtl2 2.1.4 以上
- SDR、8bit、ステレオ音声

リリース package に `Processing.NDI.Lib.x64.dll` を同梱します。

## インストール

1. GitHub Releases の `aviutl2-network-video-output-v*.au2pkg.zip` を入手します。
2. AviUtl2 のパッケージ機能、または AviUtl2 Catalog からインストールします。
3. 手動の場合は zip を展開し、`Plugin/aviutl2_ndi_live_output.aux2` と `Plugin/Processing.NDI.Lib.x64.dll` を AviUtl2 の `Plugin` フォルダへ、言語ファイルを `Language` へコピーします。

アンインストール時は、配置した `.aux2`、同梱 NDI® runtime DLL、付属データ (`Plugin/aviutl2_ndi_live_output/`)、言語ファイルを削除してください。

## 使い方

今開いているシーンだけ送れます。送りたいシーンを AviUtl2 で開いてください。

1. 送りたいシーンを開きます。
2. プラグインのウィンドウでソース名などの送信設定を確認し、**描画開始** を押します。描画中はそのシーンを編集しないでください。
3. 完了すると **CUE** が点灯し、先頭の映像を送出し始めます。受信側では `パソコンのNDI名 (ソース名)` で受けます。先頭の名前は PC 側の NDI® 名で、プラグインから変更できません。
4. **CUE** ボタンで送出・再生を開始します。再生が終わると最後の映像のまま送出し、CUE をもう一度押すと最初から再生します。
5. **停止** で描画データを捨てて、送出を終えます。

無圧縮なので、1080p は1フレームあたり約 8 MB 使います。

## 開発

Windows x64 で NDI 6 SDK と LLVM/Clang（bindgen 用）が必要です。

```powershell
# NDI 6 SDK: https://ndi.video/type/developer/
# 既定パス: C:\Program Files\NDI\NDI 6 SDK
# または NDI_SDK_DIR を設定します。
rustup toolchain install 1.97.1
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked -- --test-threads=1
cargo build --release --locked --target x86_64-pc-windows-msvc
./scripts/package.ps1
```

依存は `Cargo.lock` で固定しています。`grafton-ndi` は `1.0.0`（`default-features = false`）、`aviutl2` / `aviutl2-eframe` は `0.43.0` です。

## ライセンス

プラグイン本体は MIT License です。第三者 crate は `THIRD_PARTY_NOTICES.md` を参照してください。

同梱 NDI® runtime (`Processing.NDI.Lib.x64.dll`) には `NDI_TERMS.txt` と `Processing.NDI.Lib.Licenses.txt` が適用されます。
