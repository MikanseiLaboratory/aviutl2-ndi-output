# AviUtl2 Network Video Output

AviUtl2 の現在シーンをメモリへ描画し、[NDI®](https://ndi.video/) でキュー送出する汎用プラグイン (`.aux2`) です。

NDI® is a registered trademark of Vizrt NDI AB.

## できること

- 現在開いているシーンを RAM へベイクし、先頭フレーム固定で NDI® 送出を開始します。
- **CUE** で壁時計のシーン fps に従って再生します。AviUtl2 の再生ボタンは使いません。
- 遅れた場合は溜めずに最新位置へ進みます。再生後は最終フレームでホールドします。
- 8-bit RGBA 映像（アルファ無効時は RGBX）とステレオ planar f32 音声を送信します。

## 動作条件

- Windows x64
- AviUtl2 2.1.4 以上
- SDR、8-bit RGBA、planar f32 ステレオ

リリース package に `Processing.NDI.Lib.x64.dll` を同梱します。

## インストール

1. GitHub Releases の `aviutl2-ndi-output-v*.au2pkg.zip` を入手します。
2. AviUtl2 のパッケージ機能、または AviUtl2 Catalog からインストールします。
3. 手動の場合は zip を展開し、`Plugin/aviutl2_ndi_live_output.aux2` と `Plugin/Processing.NDI.Lib.x64.dll` を AviUtl2 の `Plugin` フォルダへ、言語ファイルを `Language` へコピーします。

アンインストール時は、配置した `.aux2`、同梱 NDI® runtime DLL、付属データ (`Plugin/aviutl2_ndi_live_output/`)、言語ファイルを削除してください。

## 使い方

1. AviUtl2 側で出したいシーンを開きます。SDK にシーン一覧やタブ切替はないため、描けるのは今開いているシーンだけです。
2. ドッキングウィンドウでソース名などの送信設定を確認し、**描画開始** を押します。ベイク中はそのシーンを編集しないでください。
3. 完了すると **CUE** が点灯し、先頭フレーム固定で NDI® 送出が始まります。受信側では `パソコンのNDI名 (ソース名)` を購読します。先頭は PC の NDI® マシン名で、プラグインから置き換えることはできません。
4. **CUE** で再生します。終了後は最終フレームでホールドし、CUE を再押しすると先頭から再生します。
5. **停止** でバッファを破棄し、送出を終了します。

無圧縮 RGBA のため 1080p は約 8 MB/frame です。

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
