# Tetorica VGM Analyzer — Tauri

[hello_ymfm_wasm](https://github.com/kyorohiro/hello_ymfm_wasm) で開発されている **VGM Analyzer の Tauri 版**です。VGM / VGZ の解析・再生機能をデスクトップアプリとして利用でき、左上のファイルメニュー、最近開いたファイル／フォルダーの履歴とPIN、ウィンドウの最前面表示を追加しています。

上流の VGM Analyzer の release ZIP を取り込み、展開した `dist/` を Git 管理する Tauri 2 シェルです。ローカルと GitHub Actions は、同じコミットの `dist/` をアプリに組み込みます。Analyzer の JavaScript / HTML / WASM は書き換えず、このリポジトリではデスクトップ向けの機能とアプリのビルド・配布を管理します。

## 起動

Node.js / npm、Python 3、Rust と [Tauri の OS 別ビルド環境](https://v2.tauri.app/start/prerequisites/) が必要です。最初の対象は macOS です。

```sh
npm ci
npm run dev
```

チェックアウトに Analyzer 本体（`dist/`）が含まれるため、起動・ビルド時の ZIP ダウンロードは不要です。`release.lock.json` は取り込み元の記録と、展開内容の整合性検査に使います。ZIP 自体は Git に含めません。

## Analyzer 本体を更新する

手動で用意した Analyzer の release ZIP のパスを指定します。
ZIP の置き場所・ファイル名は自由です。相対パスはコマンドを実行したディレクトリから解釈します。

```sh
npm run import:release -- ./xxx.zip
npm test
npm run dev
```

取り込みで `dist/` と `release.lock.json` が更新されます。バージョン指定は不要です。
ZIP の SHA-256 と展開した各ファイルのハッシュを記録します。

動作確認後、`dist/` と `release.lock.json` を同じコミットに含めてから `npm run build` します。
ZIP 自体は Git に含めません。dev / build 前の `--check` はチェックアウト済み `dist/` の整合性を検査し、ZIP の取得や取り込みは行いません。
Windows でもファイルのハッシュが変わらないよう、`.gitattributes` で `dist/` の改行変換を無効にしています。

macOS のビルド結果は `src-tauri/target/release/bundle/macos/` に出力されます。ローカルの既定ビルドは app のみです。CI では下記の各 OS 向け配布形式を指定します。署名・公証・自動更新は未対応です。

## ファイルメニュー・Recent・PIN

画面左上の **☰** から利用します。履歴はOSのアプリデータディレクトリ内の `recent-library.json` に保存され、再起動後も残ります。

- **Open Files…**：VGM / VGZ / S98を複数選択できます。複数ファイルは一組の履歴として記録します。
- **Open Folder…**：サブフォルダーを含む曲をプレイリストに読み込みます。フォルダーを開き直すたびに走査するので、追加・削除した曲も反映します。シンボリックリンクは走査しません。
- **Pinned / Recent**：項目をクリックすると再度開きます。☆でPIN、★で解除。通常履歴は直近20件、PIN済み項目は件数制限の対象外です。
- **×**：履歴から取り除きます。ファイルは削除しません。移動・削除などで開けない項目も自動では取り除きません。
- 選択後は既存のプレイリストに曲が入り、**Play** で再生します。メニューの履歴に登録されるのは、このメニューから開いた項目です。従来のHTMLファイル選択・ドロップはそのまま利用できます。

一度に10,000曲、1ファイル256 MiBを上限としています。対応する上流版では各曲の再生・解析時に読み込みます。旧ZIPとの互換経路では全曲を `File` として読み込むため、合計256 MiBを上限とします。

## ウィンドウの最前面表示

左上の **☰ → Always on Top** で最前面表示を切り替えます。On / Offで現在の状態を表示します。ネイティブメニューに独自のAnalyzer項目は追加しません。起動時は OFF で、設定は保存しません。

標準メニューを残し、コピー・貼り付けなどの操作を維持します。HTML のファイルドロップを使うため Tauri の独自ドラッグ＆ドロップ処理は無効にしています。

## 境界

- このリポジトリ：ZIP取り込み、ハンバーガーメニュー、ネイティブファイル選択・読み込み、履歴・PIN保存、ウィンドウ、配布設定。
- 上流 Analyzer：解析、表示、音声、エクスポート。
- 将来の MCP：**Playground のみ**を対象とする予定。現時点ではサーバーも操作 API も実装していません。

`desktop/desktop-interface.js` をTauriの初期化スクリプトとして追加します。ZIP内のファイルの差し替えは行いません。通常のブラウザーには追加されません。

新しい上流版は `desktop_interface.js` の `connectDesktop({openFiles})` で既存プレイリストの受け口を登録します。接続仕様は `window.__tetoricaDesktop.version === 1`。旧ZIPでは既存の `fileInput` に標準の `FileList` とchangeイベントを渡します。

ファイル選択、履歴取得・更新、読み込みは限定したRustコマンドを使います。ローカルのmainウィンドウからのみ受け付け、読み込み対象も選択済みの曲のトークンに限定します。Tauriの公開グローバルAPIは有効にしません。履歴のPINと、ウィンドウの最前面表示のPINは別の設定です。既存 release のスクリプト・WASM 読み込みをそのまま使うため、シェル独自の CSP はまだ追加していません。

MCP に進む場合は、Playground 本体に UI と共有する操作窓口（コード取得・更新、実行、停止、状態・エラー取得）を設け、Tauri 側から接続する方針です。ZIP 内の JavaScript への後付けパッチは避け、対応する release と接続仕様のバージョンを合わせます。

## 確認

```sh
npm test
python3 scripts/import_release.py --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo build --locked --manifest-path src-tauri/Cargo.toml
```

自動テストではPIN保持・再読み込み・履歴保存を確認します。

デスクトップ上で別途確認する項目：ハンバーガーメニューからのファイル／フォルダー選択、履歴から再度開く操作、PINと再起動後の保持、VGM/VGZ の選択・ドロップ、再生・停止・シーク、音声の途切れ、ファイル保存、PIN 切替。ブラウザーで動作することだけでは WebView での音声・ダウンロード互換性を保証できません。

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

Node 22、Python 3.12、Rust 1.91.1 を使用し、npm / Cargo の lockfile を維持します。Actions はチェックアウト済みの `dist/` を `release.lock.json` の各ファイルのハッシュと照合します。Analyzer 本体を外部から取得しません。上流を更新する際は、`npm run import:release -- ./xxx.zip` で新 ZIP を取り込み、動作確認後に `dist/` と `release.lock.json` を一緒にコミットしてください。タグもそのコミットに付けます。`release.source.json` と `fetch_release.py` は廃止しました。

タグはこの Tauri アプリのバージョンです。タグ作成前に `package.json` / `package-lock.json` と `src-tauri/Cargo.toml` / `Cargo.lock`、`tauri.conf.json` のバージョンを揃えてください。署名用 Secret は設定していないため、生成物は未署名です。

CI 自体は push 後の初回実行で確認が必要です。各 OS での再生・保存・PIN の動作確認はビルド成功とは別に行ってください。
