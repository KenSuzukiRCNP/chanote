# chanote

茶道のお稽古を記録するデスクトップアプリ。
A desktop app for keeping a tea-ceremony (茶道) practice log.

- 日々のお稽古を「日付・場所・先生」+「点前ごとの行(点前・亭主・自分の役割)」で記録
- 過去の記録も遡って入力・修正。日付が月までしか分からない、棚が不明、といった曖昧さをそのまま残せる
- 裏千家の点前一覧を内蔵。自分の呼び名(「長板濃茶」など)も登録できる
- データは 1 つの SQLite ファイル。外部サービスなし
- 日本語 / English UI。茶道用語は日本語・ローマ字・併記を選択

設計の詳細は [docs/chanote-design.md](docs/chanote-design.md) を参照。

## Status

Phase 3 まで — 記録の追加/編集/一覧(参加者・席順・道具・付物・写真)、リスト編集(呼び名の登録を含む)、
統計(カバレッジ表・久しくやっていない点前・時系列・クロス表)、JSON/CSV エクスポート、JSON 再読込、バックアップ。
写真込みの zip バックアップと復元、HEIC 取り込み、配布パッケージ(GitHub Actions)、
複数人で使うプロファイル、直近重視の頻度ソート、分類・先生・場所・人別の統計、道具の分類とメモ。
公開手順は [docs/PUBLISHING.md](docs/PUBLISHING.md)、ロードマップは設計書 §8。

## 使う(macOS)

[Releases](../../releases) から自分の Mac に合う `.dmg` をダウンロードします
(`aarch64` = Apple Silicon / M1 以降、`x64` = Intel)。開いて `chanote.app` を
「アプリケーション」フォルダへドラッグしてください。

初回だけ、右クリック(control+クリック)→「開く」→「開く」で起動します。
これは配布物に Apple の署名を付けていないためで、2回目以降は普通にダブルクリックで開けます。
「壊れているため開けません」と出る場合はターミナルで次を一度実行してください:

```sh
xattr -cr /Applications/chanote.app
```

Windows は `.exe`(インストーラ)、Linux は `.AppImage` または `.deb` を使います。

## 開発

必要なもの: [Rust](https://rustup.rs) (stable)、Node.js 20+、Xcode Command Line Tools。
Windows/Linux は [Tauri の前提条件](https://tauri.app/start/prerequisites/) を参照。

```sh
npm install
npm run tauri dev        # アプリを起動(初回は Rust のビルドに数分)
npm run tauri build      # .app / .dmg を作る(src-tauri/target/release/bundle/)
```

フロントエンドだけをブラウザで試す(Rust なし、データはメモリ上のモック):

```sh
npm run dev              # http://localhost:1420
```

テスト:

```sh
cargo test -p chanote-core   # DB・パック・記録の往復テスト
npm run check                # svelte-check / TypeScript
```

データファイルの場所: `~/Library/Application Support/jp.chanote.app/chanote.sqlite`(macOS)。
複数人で使う場合は設定画面で「使う人」を追加すると `profiles/<名前>/` に別のデータができます。
写真は同じフォルダの `attachments/` にコピーされます。環境変数 `CHANOTE_DB=/path/to/file.sqlite` で上書きできます。
写真は JPEG/PNG/WebP/GIF/TIFF/BMP に対応。HEIC(iPhone)は macOS では自動で JPEG に変換して取り込みます(内蔵の `sips` を使用)。
バックアップ(設定 → バックアップを保存)はデータベースと写真フォルダをまとめた zip で、「復元」で別の Mac にも移せます。

アイコン: 元画像は `assets/icon.png`(1024×1024、円の外は透明)。変更したら `npm run tauri icon assets/icon.png` で `src-tauri/icons/` を作り直します(android/ios フォルダは不要なので消してよい)。

## 配布パッケージを作る

### 手元で作る(まずはこれ)

```sh
npm run tauri build
```

数分で `src-tauri/target/release/bundle/` に次ができます:

- `macos/chanote.app` — アプリ本体
- `dmg/chanote_0.2.0_aarch64.dmg` — 配布用ディスクイメージ(Apple Silicon でビルドした場合)

Intel Mac 向けも同じ Mac で作れます(初回は `rustup target add x86_64-apple-darwin`):

```sh
npm run tauri build -- --target x86_64-apple-darwin
```

`.dmg` をそのまま渡せば、相手は上の「使う」の手順で起動できます。

### GitHub Releases で自動的に作る

`.github/workflows/release.yml` が入っています。`git tag v0.1.0 && git push --tags` すると
GitHub Actions が macOS(両チップ)・Windows・Linux の全部をビルドし、下書きの Release に添付します。
Releases ページで内容を確認して「Publish」すれば公開です。バージョン番号は
`scripts/bump-version.sh 0.3.0` で3か所まとめて上げてからタグを打ちます。

### 署名と notarize(任意)

署名なしでも配れますが、上記の「右クリック→開く」が必要です。それを無くすには
Apple Developer Program(年額)に登録し、Developer ID Application 証明書を作って、
リポジトリの Secrets に `APPLE_CERTIFICATE`(.p12 を base64)、`APPLE_CERTIFICATE_PASSWORD`、
`APPLE_SIGNING_IDENTITY`、`APPLE_ID`、`APPLE_PASSWORD`(App 用パスワード)、`APPLE_TEAM_ID` を
登録します。ワークフローはそれらがあれば自動で署名と notarize を行います。
詳細は Tauri のドキュメント「macOS Code Signing」を参照。

## 構成

```
src/                  Svelte 5 + TypeScript + Tailwind のフロントエンド
  lib/api.ts          Tauri コマンドの型付きラッパー(ブラウザではモックに切替)
  lib/i18n.svelte.ts  UI 文字列と用語表示モード
  routes/             画面(記録一覧・編集・統計・リスト編集・設定)
src-tauri/            Tauri シェル(コマンド定義のみ。ロジックは持たない)
crates/chanote-core/  SQLite スキーマ・マイグレーション・記録/語彙の操作・パック読込
packs/urasenke.yaml   裏千家の点前・棚・炉風炉の一覧(流派パック)
locales/              ja.json / en.json
docs/                 設計書
```

## 流派パックを足す・直す

`packs/urasenke.yaml` を編集して `version` を上げると、次回起動時に取り込まれます。
ユーザーがアプリ内で追加した項目には触れません。他流派のパックも同じ形式で書けます
(現状は裏千家パックのみ同梱。複数パックの切替は今後)。

## License

MIT
