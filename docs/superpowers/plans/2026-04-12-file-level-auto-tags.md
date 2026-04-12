# ファイル単位の自動タグ機能 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Markdown ファイル先頭の YAML front matter で `tags` を宣言すると、そのファイル内の全タスクにタグが自動付与される機能を追加する。

**Architecture:** パース処理は `parser-core` (Rust) に実装し、WASM 経由で VS Code 拡張が利用する。CLI は直接呼び出す。タスクごとのタグ抽出 (`extract_tags`) と合算する形で既存機能を拡張する。

**Tech Stack:** Rust (parser-core, parser-wasm, cli), TypeScript (VS Code 拡張), serde_yml (YAML パーサー), wasm-bindgen

関連スペック: `docs/superpowers/specs/2026-04-12-file-level-auto-tags-design.md`

---

## File Structure

- **Modify** `parser-core/Cargo.toml` — `serde_yml` 依存追加
- **Modify** `parser-core/src/lib.rs` — `FrontMatter` 構造体、`parse_front_matter`、`extract_file_tags` 関数とテストを追加
- **Modify** `parser-wasm/src/lib.rs` — `extractFileTags` の WASM エクスポート追加
- **Modify** `src/parser.ts` — `extractFileTags` の re-export
- **Modify** `src/extension.ts` — `extractFileTags` の re-export（他のパーサー関数と同列）
- **Modify** `src/tagTreeProvider.ts` — ファイル自動タグとタスクタグの合算
- **Modify** `cli/src/main.rs` — `filter_tree_by_tag` にファイル自動タグを渡す
- **Modify** `CLAUDE.md`, `README.md`, `cli/AGENTS.md` — 新記法の説明追加

---

## Task 1: `parse_front_matter` を parser-core に追加（TDD）

**Files:**
- Modify: `parser-core/Cargo.toml`
- Modify: `parser-core/src/lib.rs`

- [ ] **Step 1: `serde_yml` 依存を追加**

`parser-core/Cargo.toml` の `[dependencies]` を以下に更新:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
regex = "1"
serde_yml = "0.0.12"
```

- [ ] **Step 2: 失敗するテストを書く**

`parser-core/src/lib.rs` の `mod tests` の末尾（既存のテスト郡の最後、`}` の直前）に追加:

```rust
    // --- parse_front_matter tests ---

    #[test]
    fn test_parse_front_matter_basic() {
        let l = lines(&[
            "---",
            "tags:",
            "  - projectA",
            "  - work",
            "---",
            "",
            "- [ ] タスク",
        ]);
        let fm = parse_front_matter(&l).expect("front matter should parse");
        assert_eq!(fm.tags, Some(vec![s("projectA"), s("work")]));
    }

    #[test]
    fn test_parse_front_matter_none_when_no_leading_delimiter() {
        let l = lines(&["", "---", "tags:", "  - work", "---"]);
        assert!(parse_front_matter(&l).is_none());
    }

    #[test]
    fn test_parse_front_matter_none_when_unclosed() {
        let l = lines(&["---", "tags:", "  - work", "- [ ] タスク"]);
        assert!(parse_front_matter(&l).is_none());
    }

    #[test]
    fn test_parse_front_matter_empty_body() {
        let l = lines(&["---", "---"]);
        let fm = parse_front_matter(&l).expect("empty front matter is valid");
        assert!(fm.tags.is_none());
    }

    #[test]
    fn test_parse_front_matter_invalid_yaml() {
        let l = lines(&["---", "tags: [unclosed", "---"]);
        assert!(parse_front_matter(&l).is_none());
    }
```

- [ ] **Step 3: テストを実行して失敗を確認**

Run: `mise run test-rust`
Expected: FAIL — `parse_front_matter` が未定義のため compile error。

- [ ] **Step 4: 実装を書く**

`parser-core/src/lib.rs` の `// === Tag extraction ===` セクションの直前に追加:

```rust
// === Front matter ===

#[derive(serde::Deserialize, Debug, Default)]
pub struct FrontMatter {
    #[serde(default)]
    pub tags: Option<Vec<serde_yml::Value>>,
}

pub fn parse_front_matter(lines: &[String]) -> Option<FrontMatterParsed> {
    if lines.is_empty() || lines[0].trim_end() != "---" {
        return None;
    }
    let mut end: Option<usize> = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim_end() == "---" {
            end = Some(i);
            break;
        }
    }
    let end = end?;
    let body = lines[1..end].join("\n");
    if body.trim().is_empty() {
        return Some(FrontMatterParsed { tags: None });
    }
    let fm: FrontMatter = serde_yml::from_str(&body).ok()?;
    let tags = fm.tags.map(|vs| {
        vs.into_iter()
            .filter_map(|v| match v {
                serde_yml::Value::String(s) => Some(s),
                serde_yml::Value::Number(n) => Some(n.to_string()),
                serde_yml::Value::Bool(b) => Some(b.to_string()),
                _ => None,
            })
            .collect::<Vec<String>>()
    });
    Some(FrontMatterParsed { tags })
}

#[derive(Debug, PartialEq)]
pub struct FrontMatterParsed {
    pub tags: Option<Vec<String>>,
}
```

- [ ] **Step 5: テストを実行して合格を確認**

Run: `mise run test-rust`
Expected: PASS（追加した 5 テストすべて）

- [ ] **Step 6: Commit**

```bash
git add parser-core/Cargo.toml parser-core/Cargo.lock parser-core/src/lib.rs Cargo.lock
git commit -m "feat(parser-core): front matter パース関数を追加します"
```

（注: `Cargo.lock` はワークスペースルートにあるので、そちらが更新されている想定）

---

## Task 2: `extract_file_tags` 関数を追加（TDD）

ファイル全行を受け取り、front matter から自動タグを返す公開関数。タグ先頭の `#` を剥がし、空文字列は除外する。

**Files:**
- Modify: `parser-core/src/lib.rs`

- [ ] **Step 1: 失敗するテストを書く**

`parser-core/src/lib.rs` の `mod tests` に追加（`parse_front_matter` テスト群の後に）:

```rust
    // --- extract_file_tags tests ---

    #[test]
    fn test_extract_file_tags_basic() {
        let l = lines(&[
            "---",
            "tags:",
            "  - projectA",
            "  - work",
            "---",
            "- [ ] タスク",
        ]);
        assert_eq!(extract_file_tags(&l), vec![s("projectA"), s("work")]);
    }

    #[test]
    fn test_extract_file_tags_none() {
        let l = lines(&["- [ ] タスク"]);
        let empty: Vec<String> = vec![];
        assert_eq!(extract_file_tags(&l), empty);
    }

    #[test]
    fn test_extract_file_tags_strips_hash_prefix() {
        let l = lines(&["---", "tags:", "  - \"#projectA\"", "  - work", "---"]);
        assert_eq!(extract_file_tags(&l), vec![s("projectA"), s("work")]);
    }

    #[test]
    fn test_extract_file_tags_skips_blank_entries() {
        let l = lines(&[
            "---",
            "tags:",
            "  - projectA",
            "  - \"\"",
            "  - \"   \"",
            "  - work",
            "---",
        ]);
        assert_eq!(extract_file_tags(&l), vec![s("projectA"), s("work")]);
    }

    #[test]
    fn test_extract_file_tags_scalar_tags_field() {
        // tags が配列でなく文字列の場合は空を返す
        let l = lines(&["---", "tags: projectA", "---"]);
        let empty: Vec<String> = vec![];
        assert_eq!(extract_file_tags(&l), empty);
    }

    #[test]
    fn test_extract_file_tags_mid_file_delimiter_ignored() {
        let l = lines(&[
            "- [ ] タスク",
            "---",
            "tags:",
            "  - work",
            "---",
        ]);
        let empty: Vec<String> = vec![];
        assert_eq!(extract_file_tags(&l), empty);
    }
```

- [ ] **Step 2: テストを実行して失敗を確認**

Run: `mise run test-rust`
Expected: FAIL — `extract_file_tags` が未定義のため compile error。

- [ ] **Step 3: 実装を書く**

`parser-core/src/lib.rs` の `// === Tag extraction ===` セクション末尾（`extract_tags` 関数の後）に追加:

```rust
pub fn extract_file_tags(lines: &[String]) -> Vec<String> {
    let Some(fm) = parse_front_matter(lines) else {
        return Vec::new();
    };
    let Some(tags) = fm.tags else {
        return Vec::new();
    };
    tags.into_iter()
        .map(|t| t.trim_start_matches('#').trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}
```

- [ ] **Step 4: テストを実行して合格を確認**

Run: `mise run test-rust`
Expected: PASS

- [ ] **Step 5: 既存パースに影響がないことを確認**

Run: `mise run test-rust`
Expected: すべての既存テストも PASS（front matter 行が既存の `parse_tasks_internal` / `parse_all_dates_internal` を壊していないこと）

- [ ] **Step 6: Commit**

```bash
git add parser-core/src/lib.rs
git commit -m "feat(parser-core): extract_file_tags 関数を追加します"
```

---

## Task 3: WASM ブリッジを追加

**Files:**
- Modify: `parser-wasm/src/lib.rs`

- [ ] **Step 1: WASM エクスポートを追加**

`parser-wasm/src/lib.rs` の末尾に追加:

```rust
#[wasm_bindgen(js_name = "extractFileTags")]
pub fn extract_file_tags(lines_js: JsValue) -> JsValue {
    let lines: Vec<String> = serde_wasm_bindgen::from_value(lines_js).unwrap_or_default();
    let tags = parser_core::extract_file_tags(&lines);
    serde_wasm_bindgen::to_value(&tags).unwrap()
}
```

- [ ] **Step 2: WASM をビルドして成功を確認**

Run: `mise run build-wasm`
Expected: ビルド成功、`src/pkg/parser_wasm.d.ts` に `extractFileTags` が生成される。

- [ ] **Step 3: 生成された型定義を確認**

Run: `grep extractFileTags src/pkg/parser_wasm.d.ts`
Expected: 関数シグネチャが出力される（例: `export function extractFileTags(lines_js: any): any;`）

- [ ] **Step 4: Commit**

```bash
git add parser-wasm/src/lib.rs
git commit -m "feat(parser-wasm): extractFileTags を WASM から公開します"
```

---

## Task 4: TypeScript 側で `extractFileTags` を re-export

**Files:**
- Modify: `src/parser.ts`

`extractTags` は `extension.ts` からは export されておらず、`tagUtils.ts` が `parser.ts` から再エクスポートして `tagTreeProvider.ts` で利用する形。同じパターンに従う。`tagUtils.ts` の変更は Task 5 で行う。

- [ ] **Step 1: `src/parser.ts` に import と re-export を追加**

`src/parser.ts` 冒頭の import 文を更新:

```typescript
import {
	parseTasks as wasmParseTasks,
	parseTasksAllDates as wasmParseTasksAllDates,
	buildTreeData as wasmBuildTreeData,
	buildScheduleData as wasmBuildScheduleData,
	extractTags as wasmExtractTags,
	extractFileTags as wasmExtractFileTags,
} from './pkg/parser_wasm';
```

ファイル末尾（既存の `extractTags` 関数の後）に追加:

```typescript
export function extractFileTags(lines: string[]): string[] {
	return wasmExtractFileTags(lines) as string[];
}
```

- [ ] **Step 2: 型チェック**

Run: `mise run check`
Expected: TypeScript type-check / lint ともに PASS

- [ ] **Step 3: Commit**

```bash
git add src/parser.ts
git commit -m "feat: extractFileTags を parser.ts から公開します"
```

---

## Task 5: `tagTreeProvider` でタグを合算

**Files:**
- Modify: `src/tagTreeProvider.ts`

`collectAllTaggedTasks()` で、ファイル単位で 1 回だけ `extractFileTags(lines)` を呼び、各タスクの `extractTags(task.text)` と合算する。合算後の配列が空のタスクはスキップする。

- [ ] **Step 1: import を更新**

`src/tagTreeProvider.ts` の import 文を変更:

```typescript
import { extractTags, extractFileTags } from './tagUtils';
```

（`extractFileTags` は `tagUtils.ts` にも再エクスポートが必要。ただし現状 `tagUtils.ts` は `parser.ts` からの re-export だけなので、下記 Step 2 で対応する）

- [ ] **Step 2: `src/tagUtils.ts` を更新**

ファイルを以下に書き換え:

```typescript
export { extractTags, extractFileTags } from './parser';
```

- [ ] **Step 3: `collectAllTaggedTasks()` の処理を修正**

`src/tagTreeProvider.ts` の `collectAllTaggedTasks` メソッド内、既存の以下のブロック:

```typescript
for (const fileUri of allFileUris) {
    const doc = await vscode.workspace.openTextDocument(fileUri);
    const lines: string[] = [];
    for (let i = 0; i < doc.lineCount; i++) {
        lines.push(doc.lineAt(i).text);
    }
    const tasksInFile = parseTasksAllDates(lines);
```

の直後（`const relativePath = ...` の前）に、ファイル自動タグを取得する処理を追加:

```typescript
    const fileTags = extractFileTags(lines);
```

さらに、既存のタグ合算ロジック:

```typescript
for (const task of seenTasks.values()) {
    const tags = extractTags(task.text);
    if (tags.length === 0) {
        continue;
    }
```

を以下に置き換え:

```typescript
for (const task of seenTasks.values()) {
    const textTags = extractTags(task.text);
    const mergedTags = [...fileTags, ...textTags.filter(t => !fileTags.includes(t))];
    if (mergedTags.length === 0) {
        continue;
    }
```

そして、この直後のタグループ `for (const tag of tags) {` を `for (const tag of mergedTags) {` に変更する。

- [ ] **Step 4: 型チェックとビルド**

Run: `mise run check`
Expected: PASS

- [ ] **Step 5: 動作確認（手動）**

1. `$HOME/taski` 配下に一時的に以下の内容の `tagtest.md` を作る:

```markdown
---
tags:
  - autoTag
---

- [ ] 自動タグだけのタスク
- [ ] 本文タグつき #extra
```

2. VS Code で拡張をリロード（`Developer: Reload Window`）
3. `TASKI: タグ別` ビューを開き、`#autoTag` に両タスクが、`#extra` に 2 つ目のタスクが集計されていることを確認
4. 確認後、`tagtest.md` を削除

- [ ] **Step 6: Commit**

```bash
git add src/tagTreeProvider.ts src/tagUtils.ts
git commit -m "feat: タグビューで front matter 自動タグを合算します"
```

---

## Task 6: CLI の `filter_tree_by_tag` で自動タグを考慮

**Files:**
- Modify: `cli/src/main.rs`

`filter_tree_by_tag` に `file_tags_by_uri: &HashMap<String, Vec<String>>` を渡し、タスクのタグ判定時にファイル自動タグも含めるようにする。

- [ ] **Step 1: 失敗するテストを書く**

`cli/src/main.rs` の `#[cfg(test)] mod tests` に追加（既存の `test_filter_tree_by_tag` の後）:

```rust
    #[test]
    fn test_filter_tree_by_tag_with_file_tags() {
        use parser_core::{TreeDateGroup, TreeFileGroup, TreeTaskData};
        use std::collections::HashMap;

        let tree = vec![TreeDateGroup {
            date_key: "2026-04-12".to_string(),
            label: "今日".to_string(),
            is_today: true,
            completed_count: 0,
            total_count: 1,
            file_groups: vec![TreeFileGroup {
                file_name: "projectA.md".to_string(),
                file_uri: "/projectA.md".to_string(),
                tasks: vec![TreeTaskData {
                    is_completed: false,
                    text: "本文にタグなしタスク".to_string(),
                    file_uri: "/projectA.md".to_string(),
                    line: 3,
                    log: String::new(),
                    date: "2026-04-12".to_string(),
                    context: vec![],
                }],
            }],
        }];

        let mut file_tags = HashMap::new();
        file_tags.insert("/projectA.md".to_string(), vec!["projectA".to_string()]);

        let filtered = filter_tree_by_tag(tree, "projectA", &file_tags);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].file_groups[0].tasks.len(), 1);
    }
```

また、既存の `test_filter_tree_by_tag` と `test_filter_tree_by_tag_no_match` の `filter_tree_by_tag(tree, "work")` 呼び出しを、新シグネチャに合わせて更新:

```rust
let filtered = filter_tree_by_tag(tree, "work", &std::collections::HashMap::new());
```

```rust
let filtered = filter_tree_by_tag(tree, "nonexistent", &std::collections::HashMap::new());
```

- [ ] **Step 2: テストを実行して失敗を確認**

Run: `mise run test-rust`
Expected: FAIL（compile error または新テスト失敗）

- [ ] **Step 3: `filter_tree_by_tag` のシグネチャを更新**

`cli/src/main.rs` の `filter_tree_by_tag` 関数を以下に置き換え:

```rust
fn filter_tree_by_tag(
    tree: Vec<TreeDateGroup>,
    tag: &str,
    file_tags_by_uri: &std::collections::HashMap<String, Vec<String>>,
) -> Vec<TreeDateGroup> {
    tree.into_iter()
        .filter_map(|mut date_group| {
            date_group.file_groups = date_group
                .file_groups
                .into_iter()
                .filter_map(|mut file_group| {
                    let empty: Vec<String> = Vec::new();
                    let file_tags = file_tags_by_uri
                        .get(&file_group.file_uri)
                        .unwrap_or(&empty);
                    file_group.tasks.retain(|task| {
                        file_tags.iter().any(|t| t == tag)
                            || extract_tags(&task.text).iter().any(|t| t == tag)
                    });
                    if file_group.tasks.is_empty() {
                        None
                    } else {
                        Some(file_group)
                    }
                })
                .collect();
            if date_group.file_groups.is_empty() {
                None
            } else {
                Some(date_group)
            }
        })
        .collect()
}
```

- [ ] **Step 4: import を追加**

`cli/src/main.rs` 冒頭の parser-core の use 文を更新:

```rust
use parser_core::{
    build_schedule_data_internal, build_tree_data_internal, extract_file_tags, extract_tags,
    FileInput, TreeDateGroup,
};
```

- [ ] **Step 5: `list_tasks` から file_tags を構築して渡す**

`cli/src/main.rs` の `list_tasks` 関数内、既存の `let files: Vec<FileInput> = md_files.iter().filter_map(...).collect();` の直後に追加:

```rust
    let file_tags_by_uri: std::collections::HashMap<String, Vec<String>> = files
        .iter()
        .map(|f| (f.file_uri.clone(), extract_file_tags(&f.lines)))
        .collect();
```

そして既存の `filter_tree_by_tag(tree, tag)` 呼び出しを以下に変更:

```rust
filter_tree_by_tag(tree, tag, &file_tags_by_uri)
```

- [ ] **Step 6: テストとビルドを確認**

Run: `mise run test-rust && mise run build-cli`
Expected: PASS / ビルド成功

- [ ] **Step 7: 動作確認（手動）**

1. `$HOME/taski/projectA.md` に以下を作成:

```markdown
---
tags:
  - projectA
---

- [ ] プロジェクト A のタスク
    - 2026-04-12: 作業中
```

2. 以下を実行:

```bash
./target/release/taski list --tag projectA
```

Expected: 「プロジェクト A のタスク」が表示される。

3. 確認後、`$HOME/taski/projectA.md` を削除。

- [ ] **Step 8: Commit**

```bash
git add cli/src/main.rs
git commit -m "feat(cli): list --tag で front matter 自動タグも対象にします"
```

---

## Task 7: ドキュメント更新

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `cli/AGENTS.md`

- [ ] **Step 1: `CLAUDE.md` に記法を追記**

`CLAUDE.md` の `## Task Markdown Format` セクションの末尾に追加:

````markdown
### ファイル単位の自動タグ

ファイル先頭に YAML front matter で `tags` を記述すると、そのファイル内の全タスクにタグが自動付与される（本文の `#tag` と合算）:

```markdown
---
tags:
  - projectA
  - work
---

- [ ] レビュー #urgent
    - 2026-04-12: 確認中
```

上記の場合、タスクは `#projectA` `#work` `#urgent` の 3 タグを持つ扱い。
````

- [ ] **Step 2: `README.md` にセクション追加**

`README.md` の既存のタグ関連説明の近く、または「機能」「使い方」セクション付近に、上記と同様の記法ブロックを追加する（内容は Step 1 と同じ）。

既存の構造を確認:

```bash
grep -n "^#" README.md | head -30
```

適切なセクションに 150 字程度の説明 + 上記コードブロックを追加。

- [ ] **Step 3: `cli/AGENTS.md` に言及を追加**

`cli/AGENTS.md` の `--tag` オプション説明付近（行 68 付近）に以下を追記:

```markdown
ファイル冒頭の YAML front matter で `tags: [projectA, work]` のように宣言したタグも、`--tag` フィルタの対象になる。
```

- [ ] **Step 4: 最終確認**

Run: `mise run compile`
Expected: すべてのビルドチェックが PASS

Run: `mise run test-rust`
Expected: 全テスト PASS

- [ ] **Step 5: Commit**

```bash
git add CLAUDE.md README.md cli/AGENTS.md
git commit -m "docs: ファイル単位の自動タグ機能をドキュメントに追記します"
```

---

## 完了判定

- [ ] `mise run test-rust` が全テスト PASS
- [ ] `mise run compile` が成功（type-check + lint + WASM + esbuild）
- [ ] `mise run build-cli` が成功
- [ ] 手動確認: front matter の `tags` が VS Code のタグビュー / CLI `list --tag` 両方で反映される
