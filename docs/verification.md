# 検証手順

自動化できる項目は CI と `cargo test --locked -- --test-threads=1` で実施します。AviUtl2 本体と外部受信機は手動確認です。

## 自動テスト

- `tests/media.rs`: padded pitch、RGBA/RGBX、アルファ正規化、planar f32、100 ns timecode、loop/seek 時の単調性、入力範囲エラー
- `tests/controller.rs`: 最新フレーム優先、音声 queue overflow、壁時計 playhead のスキップ
- `src/player.rs`: 先頭ホールド、最終フレーム固定、CUE 再押し
- `tests/ndi_loopback.rs`: `Finder` / `Receiver` で RGBA 映像、stereo 音声、frame rate/timecode、途中接続、切断再接続、connection count

NDI runtime が無い環境では loopback は skip します。CI は NDI 6 SDK と LLVM 20.1.8 を SHA-256 検証付きで導入します。

## キュープレイヤー（実機ゲート）

AviUtl2 2.1.4 でプラグインを load し、出したいシーンを開いたうえで **描画開始** する。完了後に先頭フレームが NDI® へ流れ、**CUE** で壁時計再生が始まり、終了後は最終フレーム固定になることを確認します。

## AviUtl2 実機

AviUtl2 2.1.4 以上、Windows x64 で次を確認します。

1. プラグインがロード・アンロードできる
2. 描画開始、進捗 `n / total`、キャンセル
3. 先頭ホールド、CUE 再生、最終ホールド、CUE 再押し
4. シーン変更後は開き直してから再ベイク
5. 解像度 / fps / sample rate 変更
6. 送出停止の反復
7. プロジェクト切替後に自動送信されないこと
8. 受信者なし、途中接続、切断

## 外部 NDI® 受信機

NDI Studio Monitor または独立した NDI receiver で、1080p59.94・48 kHz stereo を 10 分以上試験し、次を記録します。

- A/V 同期
- drop 数
- CPU 負荷
- メモリ
- 遅延

再生が遅れた場合は溜めず最新位置へジャンプすることを確認します。

## クリーン package

NDI 未導入のクリーン Windows x64 環境へ `.au2pkg.zip` だけを入れ、同梱 DLL の解決、plugin load、送受信、更新、アンインストールを確認します。
