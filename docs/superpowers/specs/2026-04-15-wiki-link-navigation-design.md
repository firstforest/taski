# Wiki リンクナビゲーション機能 設計

日付: 2026-04-15

## 目的

Markdown ドキュメント内の `[[...]]` 記法を VSCode 上でクリッカブルにし、対応するファイルへジャンプできるようにする。対応ファイルが存在しない場合は自動的に作成して開く。Rust 側にロジックを置き、CLI からも同じ解決機能を利用できるようにする。

## 非目標

- F12（Go to Definition）からのジャンプ（今回は実装しない）
- エイリアス記法 `[[foo|表示名]]`
- `[[foo#heading]]` のような heading アンカー
- エディタ起動を伴う CLI サブコマンド（AI エージェント利用を前提に、パス出力のみに留める）

## リンク記法

対象とする正規表現: `\[\[([^\[\]|]+?)\]\]`

- `[[foo]]` → `foo.md` として解決
- `[[foo.md]]` → 末尾の `.md` を剥がして `foo` として解決
- `[[YYYY-MM-DD]]`（例: `[[2026-04-14]]`）→ 日付リンク扱い
- `|`、`[`、`]` を含むものは対象外

## 解決ロジック

入力: リンク名（正規化済み）、現在編集中のファイル URI、taski がスキャン対象とする markdown URI 一覧。

1. リンク名を正規化する（末尾 `.md` を剥がし、`YYYY-MM-DD` にマッチするか判定）
2. 候補 URI を優先順位付きで走査し、拡張子を除いたファイル名が一致するものを探す
   - 優先順位: `$HOME/taski` 配下 > ワークスペース > 追加ディレクトリ（`taski.additionalDirectories`）> 開いているドキュメント
   - 最初に一致した 1 件を採用する
3. 見つかれば既存ファイルを開く
4. 見つからなければ作成先を決定する
   - 日付リンク: `$HOME/taski/journal/<YYYY>/<MM>/<YYYY-MM-DD>.md`
   - それ以外: `$HOME/taski/note/<name>.md`
5. ディレクトリを再帰的に作成し、初期内容 `# <name>\n` を書き込んでから開く

## Rust 実装 (`parser-core`)

新規モジュール `parser-core/src/wiki_link.rs` を追加する。

```rust
pub struct WikiLinkMatch { pub name: String, pub start: usize, pub end: usize }
pub struct NormalizedName { pub name: String, pub is_journal: bool }

pub fn parse_wiki_links(text: &str) -> Vec<WikiLinkMatch>;
pub fn normalize_wiki_name(raw: &str) -> NormalizedName;
pub fn resolve_wiki_link(name: &str, candidates: &[PathBuf]) -> Option<PathBuf>;
pub fn wiki_link_create_path(name: &str, is_journal: bool, taski_home: &Path) -> PathBuf;
pub fn wiki_link_initial_content(name: &str) -> String;
```

- `parse_wiki_links` は上記の正規表現でドキュメント全体から一致箇所を抽出する
- `normalize_wiki_name` は末尾 `.md` 除去と日付判定を行う
- `resolve_wiki_link` は候補 URI の順序に従って最初の一致を返す（優先順位付けは呼び出し側で行う）
- `wiki_link_create_path` は新規作成先の絶対パスを返す
- `wiki_link_initial_content` は `# <name>\n` を返す

テストは同ファイル内 `#[cfg(test)] mod tests` に Rust 側で書く。ケースとしては最低限以下を含める。

- 通常のリンク、`.md` 付きリンク、日付リンクのパース
- 候補一覧からの既存解決（優先順位による最初の 1 件採用）
- 新規作成先パスの組み立て（ジャーナル vs ノート）

## WASM 公開 (`parser-wasm`)

`parser-wasm/src/lib.rs` で以下を wasm-bindgen 経由で公開する。

- `parseWikiLinks(text: &str) -> JsValue` — `[{ name, start, end }]` を返す
- `normalizeWikiName(raw: &str) -> JsValue` — `{ name, isJournal }` を返す
- `resolveWikiLink(name: &str, candidatePaths: Vec<String>) -> Option<String>` — 一致する最初のパスを返す
- `wikiLinkCreatePath(name: &str, isJournal: bool, taskiHome: &str) -> String`
- `wikiLinkInitialContent(name: &str) -> String`

TypeScript 側の型定義は `src/parser.ts` に追加する。

## TypeScript 実装

### 新規ファイル `src/wikiLinkProviders.ts`

以下を実装する。

- `WikiLinkDocumentLinkProvider implements vscode.DocumentLinkProvider`
  - `provideDocumentLinks` で WASM の `parseWikiLinks` を呼び、各マッチを `DocumentLink` に変換する
  - `target` は独自コマンド URI `command:taski.openWikiLink?<encoded-args>` に設定する（存在しないファイルへの遷移をコマンド経由で処理するため、直接 file URI は使わない）
- 内部コマンド `taski.openWikiLink`
  - 引数: `{ name: string, fromUri: string }`
  - 処理: 正規化 → `fileScanner.findAllMarkdownUris()` で候補収集 → 優先順位ソート → `resolveWikiLink` → 無ければ `wikiLinkCreatePath` で作成先決定、ディレクトリ作成、`wikiLinkInitialContent` を書き込み → `vscode.window.showTextDocument`

### 変更ファイル

- `src/extension.ts`
  - `WikiLinkDocumentLinkProvider` を Markdown ドキュメントに対して登録
  - `taski.openWikiLink` コマンドを登録
- `src/parser.ts`
  - 新 WASM 関数の型定義を追加し再エクスポート
- `src/fileScanner.ts`
  - 既存の `findAllMarkdownUris` をそのまま利用する（変更不要）
- `package.json`
  - `contributes.commands` に `taski.openWikiLink` を追加（内部利用のため `commandPalette: false`）

### テスト

- `src/test/wikiLink.test.ts` を追加
  - 解決ロジックの VSCode 統合テスト最小限（優先順位、存在しないリンクの自動作成、ジャーナルパスの組み立て）
  - ファイル作成はテンポラリディレクトリを使う

## CLI 実装 (`cli/`)

### 新規サブコマンド `taski resolve`

```
taski resolve <name> [--no-create] [--format json]
```

- `<name>` は `[[...]]` の中身と同等の文字列
- 既定動作: 既存ファイルが見つかればそのパスを stdout に出力。無ければ作成してパスを出力
- `--no-create`: 作成を抑止。見つからない場合は stderr にエラーを出し exit code 1
- `--format json`: `{ "path": string, "created": boolean, "is_journal": boolean }` を出力

### 候補収集

CLI 側では以下を候補とする（VSCode 設定は読めないため、`$HOME/taski` 配下と現在のカレントディレクトリを対象とする）。

- `$HOME/taski` 配下の markdown ファイル（`fileScanner.ts` 相当の Rust 実装を `parser-core` または `cli` 側に用意する）
- 作成先は TypeScript 側と同じ（ジャーナルなら `$HOME/taski/journal/<YYYY>/<MM>/<YYYY-MM-DD>.md`、それ以外は `$HOME/taski/note/<name>.md`）

### 変更ファイル

- `cli/src/main.rs` — サブコマンド定義を追加
- `cli/src/commands/resolve.rs`（新規）— 実行ロジック
- 必要に応じて `parser-core` にディレクトリ走査ユーティリティを追加

## 自動作成の挙動

- 事前確認ダイアログは出さない
- 初期内容: `# <name>\n`（末尾改行あり、1 行のみ）
- 作成先ディレクトリは `fs::create_dir_all` 相当で再帰作成
- 同名ファイルが作成タイミングで既に存在した場合は上書きせず、その既存ファイルを開く

## エラーハンドリング

- 空リンク `[[]]` は無視（正規表現の `+?` で 1 文字以上にマッチ）
- `$HOME` が解決できない環境では CLI・拡張ともに解決失敗としてエラー表示
- 権限エラー等でファイル作成に失敗した場合、VSCode 側は `showErrorMessage`、CLI 側は stderr に出して exit code 1

## 受け入れ条件

- [ ] Markdown ファイル中の `[[foo]]` を Cmd+Click すると、既存の `foo.md` が優先順位に従って開く
- [ ] 対応ファイルが無い場合、`$HOME/taski/note/foo.md` が `# foo\n` で作成され開く
- [ ] `[[2026-04-14]]` は `$HOME/taski/journal/2026/04/2026-04-14.md` を作成・参照する
- [ ] `[[foo.md]]` も `[[foo]]` と同等に扱われる
- [ ] `taski resolve foo` が既存パスを stdout に出力、無ければ作成して出力する
- [ ] `taski resolve foo --no-create` は見つからない場合 exit code 1 で終了する
- [ ] `taski resolve foo --format json` が構造化出力を返す
- [ ] `parser-core` の wiki_link モジュールに Rust 単体テストが存在し、`mise run test-rust` で通る
