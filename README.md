# Tetorica VGM Analyzer — Tauri

[hello_ymfm_wasm](https://github.com/kyorohiro/hello_ymfm_wasm) で開発されている **VGM Analyzer の Tauri 版**です。VGM / VGZ の解析・再生機能をデスクトップアプリとして利用でき、ウィンドウを最前面に固定する PIN 機能を追加しています。

上流の VGM Analyzer の release ZIP をそのまま組み込む Tauri 2 シェルとして構成しています。Analyzer の JavaScript / HTML / WASM は書き換えず、このリポジトリではデスクトップ向けの機能とアプリのビルド・配布を管理します。

## 起動

Node.js / npm、Python 3、Rust と [Tauri の OS 別ビルド環境](https://v2.tauri.app/start/prerequisites/) が必要です。最初の対象は macOS です。

```sh
npm ci
# release.lock.json に記録された ZIP をローカルに用意する
npm run import:release -- /path/to/hello_ymfm_wasm_dev_itch_vgm_analyzer_v0.40.13+2.zip
npm run dev
```

初期の取り込み元は、手元の `hello_ymfm_wasm_dev_itch_vgm_analyzer_v0.40.13+2.zip` です。`python3 scripts/fetch_release.py` で、`release.source.json` の GitHub Release URL から同じ ZIP を取得できます。SHA-256 が一致しない場合は取り込みません。ZIP は Git に含めず、バージョン、ファイル名、SHA-256 と展開内容のハッシュを `release.lock.json` に記録しています。

別の release に更新するときだけバージョンを明示します。

```sh
npm run import:release -- /path/to/new-release.zip --version vX.Y.Z
npm test
npm run build
```

`--version` なしの取り込みでは、ZIP の SHA-256 が現在の記録と一致する必要があります。dev / build 前にも展開内容を検査します。`dist/` は生成物です。変更は上流で行い、新しい ZIP として取り込んでください。

macOS のビルド結果は `src-tauri/target/release/bundle/macos/` に出力されます。ローカルの既定ビルドは app のみです。CI では下記の各 OS 向け配布形式を指定します。署名・公証・自動更新は未対応です。

## PIN

ネイティブメニューの **Analyzer → PIN — Always on Top** で最前面表示を切り替えます。起動時は OFF で、設定は保存しません。macOS では画面上端のメニューバーに表示されます。

標準メニューを残し、コピー・貼り付けなどの操作を維持します。HTML のファイルドロップを使うため Tauri の独自ドラッグ＆ドロップ処理は無効にしています。

## 境界

- このリポジトリ：ZIP 取り込み、ネイティブウィンドウ、PIN、配布設定。
- 上流 Analyzer：解析、表示、音声、エクスポート。
- 将来の MCP：**Playground のみ**を対象とする予定。現時点ではサーバーも操作 API も実装していません。

PIN は Rust 側だけで処理します。Web 側へ Tauri のグローバル API や操作権限を公開しません。既存 release のスクリプト・WASM 読み込みをそのまま使うため、シェル独自の CSP はまだ追加していません。

MCP に進む場合は、Playground 本体に UI と共有する操作窓口（コード取得・更新、実行、停止、状態・エラー取得）を設け、Tauri 側から接続する方針です。ZIP 内の JavaScript への後付けパッチは避け、対応する release と接続仕様のバージョンを合わせます。

## 確認

```sh
npm test
python3 scripts/import_release.py --check
cargo check --locked --manifest-path src-tauri/Cargo.toml
```

デスクトップ上で別途確認する項目：VGM/VGZ の選択・ドロップ、再生・停止・シーク、音声の途切れ、ファイル保存、PIN 切替。ブラウザーで動作することだけでは WebView での音声・ダウンロード互換性を保証できません。

上流 ZIP に含まれるライセンス・クレジットは変更せず同梱します。本リポジトリ独自コードのライセンスは未指定です。

## アイコン

上流の `docs/icons/analyzer-512.png` を `assets/app-icon.png` に同梱しています。赤い Analyzer アイコンを macOS のアプリ・Dock、Windows 用 ICO、共通 PNG に使用します。

```sh
npm run icons
npm run build
```

元画像を更新した場合はこの手順で再生成してください。生成スクリプトはデスクトップ用のファイルだけを保存します。

## GitHub Actions

`.github/workflows/build-desktop.yml` を追加しています。

- Actions の **Build Desktop → Run workflow**：ビルドして Actions Artifacts に保存。
- `v*` タグの push：ビルドして **Draft Release** に添付。公開は手動です。
- macOS ARM64 / x86_64：DMG。
- Windows x86_64：NSIS インストーラー。
- Linux x86_64 / ARM64：DEB と AppImage。

Node 22、Python 3.12、Rust 1.91.1 を使用し、npm / Cargo の lockfile を維持します。Analyzer ZIP は `release.source.json` から取得して `release.lock.json` の SHA-256 を検証します。上流を更新する際は、ローカルで新 ZIP を `--version` 付きで取り込み、URL と lockfile を一緒に更新してください。認証不要の公開 Release URL を想定しています。

タグはこの Tauri アプリのバージョンです。タグ作成前に `package.json` / `package-lock.json` と `src-tauri/Cargo.toml` / `Cargo.lock`、`tauri.conf.json` のバージョンを揃えてください。署名用 Secret は設定していないため、生成物は未署名です。

CI 自体は push 後の初回実行で確認が必要です。各 OS での再生・保存・PIN の動作確認はビルド成功とは別に行ってください。
