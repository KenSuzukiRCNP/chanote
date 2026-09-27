# Changelog

chanote の変更履歴です。形式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に、
バージョン番号は [Semantic Versioning](https://semver.org/lang/ja/) に従います。
各版の冒頭に英語の要約を添えています。

All notable changes to chanote are listed here, newest first. Each version starts with a
one-paragraph English summary; the details are in Japanese.

## [Unreleased]

### 修正
- GitHub Actions の macOS ビルドが署名で失敗していたのを修正(ad-hoc 署名に変更)。

### 予定
- 人物・場所の入力補完(コンボボックス)
- 統計の期間の任意指定、CSV の列選択
- 流派パック(裏千家以外)の追加 ── [docs/school-packs.md](docs/school-packs.md) 参照

## [0.3.0] - 2026-09-27

**Summary.** Record filters (procedure or whole category, period, place, teacher, people,
utensils, note text), a notes toggle on the list, bigger multi-line notes, an import wizard
that can read a friend's export *as your own diary* with a per-record duplicate check,
editing of built-in procedures (with "reset to pack") and a YAML copy button, four missing
七事式 procedures, and correctly rotated photo thumbnails. Export format is now schema 2;
the database is upgraded automatically on first start.

### 追加
- **記録の絞り込み**: 点前(分類を選ぶと配下の全点前とその呼び名をまとめて)、自分が亭主の
  ものだけ、期間、場所、先生、同席した人(複数指定は AND)、使った道具(同じく AND)、
  メモの言葉。絞り込みは編集画面から戻っても保持されます。
- **メモの表示切り替え**: 一覧に「メモを表示」を追加(設定に保存)。点前で絞り込むと自動でオン。
- **読み込みウィザード**: JSON 読み込み前に全記録を一覧表示し、既存記録と一致しそうなものに
  「重複?」を付けて既定でチェックを外します。取り込む記録を選べます。
- **自分の入れ替え読み込み**: 稽古仲間の書き出しを「自分の日記として」読み込めます。
  ファイル内の人物を自分に指定すると、その人が亭主だった点前は自分の点前、客だった点前は
  自分の役割・席になり、元の記録者は指定した名前の人物として残ります。
- **点前一覧の編集画面**: 内蔵(パック由来)の点前も分類・既定値(炉/風炉・棚など)を編集
  できます。編集した行はパック更新で上書きされず、「パックの値に戻す」で元に戻せます。
- **YAML コピー**: 追加・編集した点前をパック形式でクリップボードにコピー。他流派パックや
  内蔵リストへの提案に使えます。一覧には内蔵リストの流派名を表示します。
- **裏千家パック**: 七事式に 雪月花之式・三友之式・仙遊之式・法磨之式 を追加。
  「御所籠」は「色紙点」の呼び名(別名)として登録。
- **サムネイルを作り直す**(設定): 既存の写真のサムネイルを再生成します。
- 点前を追加するとき、その場で分類を選べます。一覧に分類を表示します。

### 変更
- **点前メモ**は全幅・複数行の入力欄になり、ラベルを「先生からの指摘・気づき」に変更。
- **書き出し形式** `schema_version: 2`。点前・分類・棚の ID が `temae.<流派>.<id>` の形に
  なりました。旧形式(schema 1)の JSON はそのまま読み込めます。
- データベースは初回起動時に自動で更新されます(マイグレーション 003)。更新前に
  設定 → 「バックアップを保存」をお勧めします。

### 修正
- 写真のサムネイルが EXIF の向きを無視して横倒しになることがあった。原本には触れません。

## [0.2.0] - 2026-09-25

**Summary.** Several people can share one computer, each with their own diary. Drop-downs are
sorted by recent use. New statistics by category, teacher, place, host watched and
co-participants. Photos can be dragged onto a record and show as thumbnails in the list.
Your own seat and the orientation are filled in automatically. Utensils get categories,
free notes and pairs (chaire + shifuku).

### 追加
- **使う人(プロファイル)**: 1台のパソコンを複数人で使えます。設定で追加・改名し、
  起動時に選ぶかどうかも設定できます。日記・写真・設定は人ごとに完全に別です。
- **直近重視の並び順**: 点前・場所・人・道具のプルダウンを、最近よく使うものが上に来る順に
  (半減期 1 年の重み付き頻度)。
- **統計**: 分類別の達成バー、先生別・場所別、見た亭主別、同席者別の集計を追加。
- **写真**: 記録へのドラッグ&ドロップ、一覧でのサムネイル表示、キャプション。
- **自分の席の自動入力**: 役割を選ぶと席(正客=1 など)を補い、順勝手を既定にします。
  自分を参加者の一人として扱えるようになりました。
- **道具**: 分類(茶碗・薄器・濃茶器・仕覆・茶杓・水指・水次)、伝来・箱書などの自由メモ、
  組(茶入を選ぶと仕覆も一緒に入る)。
- `scripts/bump-version.sh` ── バージョン番号を一括で更新。

## [0.1.0] - 2026-09-23

**Summary.** First complete version: records with participants, seating, utensils, tsukimono
and photos; Urasenke procedure list with your own aliases; coverage, "not done recently",
timeline and cross-table statistics; JSON/CSV export and JSON re-import; zip backup with
photos and restore; HEIC photos; Japanese/English UI with three term modes; app icon and
release builds for macOS, Windows and Linux.

### 追加
- **記録**: 1日1件のセッション(日付は日・月・年・不明の精度、場所、稽古/講習会/本番、
  オンライン、先生、メモ)と、点前ごとの行(点前、亭主、先生、自分の役割 ── 亭主・正客・客・
  半東・水屋・見学、席順、参加者、道具、付物、メモ、写真)。
- **点前の二層構造**: 主点前 + 修飾(炉/風炉、勝手、部屋、濃茶/薄茶、棚、貴人、付物)。
  「長板濃茶」のような自分の呼び名を登録すると元の点前に展開して集計します。
- **裏千家の点前一覧**を内蔵: 平点前、小習十六ヶ条、炭手前、応用、茶箱、四ヶ伝、奥伝、七事式。
  リスト編集で追加・改名・並べ替え・非表示・削除(記録で使用中なら拒否)。
- **統計**: カバレッジ(点前×呼び名×炉風炉×役割)、久しくやっていない点前(季節で絞り込み)、
  月別の時系列(炉・風炉で色分け)、クロス表(点前×棚、場所×先生など任意の2軸)。
- **書き出し・読み込み**: JSON(完全)、CSV(1行=1点前)、JSON の再読み込み。
- **バックアップ**: 写真込みの zip を保存し、別のパソコンに復元。
- **写真**: ファイルを選んで添付、HEIC は JPEG に変換(macOS)。
- **日本語/英語 UI**。茶道用語は日本語・日本語+ローマ字・ローマ字から選択、英語の説明を
  ホバーで表示。名前は日本語かアルファベットのどちらか一方だけでも登録できます。
- アプリアイコン、macOS(Apple Silicon / Intel)・Windows・Linux 向けの配布ビルド。

[Unreleased]: https://github.com/KenSuzukiRCNP/chanote/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/KenSuzukiRCNP/chanote/releases/tag/v0.3.0
[0.2.0]: https://github.com/KenSuzukiRCNP/chanote/releases/tag/v0.2.0
[0.1.0]: https://github.com/KenSuzukiRCNP/chanote/releases/tag/v0.1.0
