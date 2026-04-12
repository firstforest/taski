# ファイル単位の自動タグ機能 設計書

作成日: 2026-04-12

## 概要

Markdown ファイル先頭の YAML front matter にタグを記述することで、そのファイル内のすべてのタスクに自動的にタグが付与されているとみなす機能を追加する。本文中の `#tag` 記法と合算される。

## 背景・動機

- 現状、タスクにタグを付けるには各タスク本文に `#tag` を書く必要がある
- プロジェクト単位でファイルを分けている場合、同じタグを繰り返し書く手間がある
- ファイル単位でタグを宣言できれば、プロジェクト横断の集計 (`taskiTagView` / CLI `--tag` フィルタ) が簡潔に書ける

## 仕様

### Front matter 記法

ファイル先頭が `---` で始まる場合のみ front matter として解釈する。YAML 配列形式で `tags` キーを記述する。

```markdown
---
tags:
  - projectA
  - work
---

- [ ] レビュー #urgent
    - 2026-04-12: 確認中
- [ ] 設計レビュー
    - 2026-04-12: 未着手
```

上記の場合、両タスクとも `#projectA` と `#work` の自動タグを持つ。1 つ目のタスクはさらに `#urgent` も持つ。

#### パース規則

- ファイル **先頭行が `---`** であり、それ以降に再度 `---` が現れる場合のみ front matter と解釈
  - ファイル途中にある `---` は無視
- `tags` キーが YAML リスト (`Sequence`) 以外の場合は自動タグなしとして扱う（警告は出さない）
- `tags` 以外のキーは無視する（将来の拡張余地として front matter 自体は受け入れる）
- 各タグ文字列の先頭 `#` は剥がす（`#projectA` / `projectA` は同一タグ扱い）
- 空白のみ / 空文字のタグ要素はスキップ
- YAML パースに失敗した場合は自動タグなしとして扱う

### タグ合算ルール

タスクが持つタグ = `extract_tags(タスク本文)` ∪ `ファイルの front matter タグ`

- 重複は排除（同じタグが本文と front matter の両方にあっても 1 件として扱う）
- 順序は front matter → 本文の順で保持する（表示順に影響する可能性があるため決定的にする）

### 影響範囲

- **VS Code 拡張 `taskiTagView`**: 自動タグのみを持つタスクも集計対象になる
- **CLI `taski list --tag <tag>`**: 自動タグも含めてフィルタする
- **CLI `taski schedule --tag <tag>`**: 同上
- **完了済みタスクの扱い**: 現状の `tagTreeProvider` の挙動（完了済みは除外）を維持する

### 既存機能との互換性

- 既存の `extract_tags(text)` API は破壊的変更なし。タスク本文からのタグ抽出のみを担当し続ける
- front matter がないファイルの挙動は従来どおり変わらない

## 実装方針

### `parser-core` crate (Rust)

新しい公開関数を追加:

```rust
pub fn extract_file_tags(lines: &[String]) -> Vec<String>
```

- ファイル全行を受け取り、front matter から自動タグを返す
- YAML パーサーは `serde_yml` を使う（`serde_yaml` の維持されている後継）
  - `parser-core` は WASM ターゲットでもビルドされるため、`wasm32-unknown-unknown` でビルドできることを実装時に確認する
  - 将来 front matter のキーが増えたときに構造体定義を足すだけで済む利点を優先
- 内部的に `parse_front_matter(lines) -> Option<FrontMatter>` を切り出し、`FrontMatter` 構造体に `serde::Deserialize` を derive する

### `parser-wasm` crate

`extract_file_tags` を `wasm-bindgen` 経由で公開:

```rust
#[wasm_bindgen(js_name = extractFileTags)]
pub fn extract_file_tags_wasm(lines: Vec<JsValue>) -> Vec<JsValue>
```

既存関数と同じインターフェース規約に揃える。

### TypeScript 側

- `src/parser.ts` に `extractFileTags(lines: string[]): string[]` を追加して re-export
- `src/extension.ts` からも re-export（既存の `extractTags` と同列）
- `src/tagTreeProvider.ts::collectAllTaggedTasks()` を修正:
  - ファイル単位で `extractFileTags(lines)` を 1 回だけ呼ぶ
  - 各タスクのタグを `union(extractTags(task.text), fileTags)` に変更
  - `if (tags.length === 0) continue;` の判定を合算後のタグ配列で行う

### CLI 側

- `cli` crate は `parser-core::extract_file_tags` を直接呼ぶ
- `taski list --tag` / `taski schedule --tag` のフィルタロジックで、タスクテキストからのタグ抽出に加えてファイル自動タグを合算
- 既存のフィルタ関数に `file_tags` を渡すように変更

## テスト

### Rust (`parser-core` のユニットテスト)

`extract_file_tags` に対して以下のケースを追加:

1. 標準的な YAML 配列 → 期待どおりタグを返す
2. front matter なし → 空配列
3. 先頭が `---` でない（空行や本文が先） → 空配列
4. `---` がファイル途中にのみある → 空配列
5. `tags` キーがない → 空配列
6. `tags` が文字列（配列でない） → 空配列
7. `tags` が空配列 → 空配列
8. タグ要素に `#` プレフィクスあり → `#` が剥がれる
9. タグ要素に空白のみの要素 → スキップされる
10. YAML パースエラー（不正な YAML） → 空配列（panic しない）
11. 終端 `---` がない（front matter が閉じていない） → 空配列

### TypeScript (既存テストスイート)

- `tagTreeProvider` の統合テストに「自動タグのみを持つタスクが集計される」ケースを追加
- `extractFileTags` の WASM ブリッジが期待通り動くかの smoke test

### CLI テスト

- `taski list --tag projectA` で front matter から自動タグ付与されたタスクがヒットすることを確認

## リリース

- `CHANGELOG` / `README` / `CLAUDE.md` の設定・機能説明セクションに front matter 記法を追記
- バージョンアップは `mise run release`（パッチバージョン）

## 将来の拡張 (本スコープ外)

- front matter の他のキー対応（例: `priority`, `defaultDue`, `owner`）
- タスク単位の YAML メタデータ（インラインメタ記法）
- ディレクトリ単位の自動タグ設定
