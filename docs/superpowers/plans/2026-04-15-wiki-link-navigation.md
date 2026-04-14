# Wiki リンクナビゲーション 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Markdown 内の `[[...]]` を VSCode 上で Cmd+Click により対応ファイルへジャンプさせ、ファイルが無ければ自動作成する。CLI からも同じ解決ロジックを `taski resolve` として使える。

**Architecture:** 解決ロジックを `parser-core` (Rust) に新規モジュール `wiki_link` として置き、WASM 経由で TypeScript に公開する。VSCode 側は `DocumentLinkProvider` + 内部コマンドで Cmd+Click を処理する。CLI は `parser-core` を直接使ってパスを標準出力に返す。

**Tech Stack:** Rust (parser-core), wasm-bindgen (parser-wasm), TypeScript (VSCode extension), clap (CLI)

**Spec:** [docs/superpowers/specs/2026-04-15-wiki-link-navigation-design.md](../specs/2026-04-15-wiki-link-navigation-design.md)

---

## File Structure

- **Create:**
  - `parser-core/src/wiki_link.rs` — 純粋ロジック（パース・正規化・解決・作成先決定・初期内容生成）と Rust 単体テスト
  - `src/wikiLinkProviders.ts` — `DocumentLinkProvider` 実装と内部コマンドハンドラ
  - `cli/src/commands/resolve.rs` — CLI サブコマンド実装
  - `src/test/wikiLink.test.ts` — 拡張統合テスト最小限

- **Modify:**
  - `parser-core/src/lib.rs` — `wiki_link` モジュール宣言と型の再 export
  - `parser-wasm/src/lib.rs` — WASM 公開関数 5 つ追加
  - `src/parser.ts` — WASM ラッパ関数と型を追加
  - `src/extension.ts` — `DocumentLinkProvider` 登録と内部コマンド登録
  - `package.json` — `taski.openWikiLink` コマンド定義追加
  - `cli/src/main.rs` — `Resolve` サブコマンド追加と `commands` モジュール宣言

---

## Task 1: Rust wiki_link モジュールの骨組みとパース

**Files:**
- Create: `parser-core/src/wiki_link.rs`
- Modify: `parser-core/src/lib.rs`

- [ ] **Step 1: 失敗テストを書く（パース）**

`parser-core/src/wiki_link.rs` を新規作成：

```rust
use regex::Regex;
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WikiLinkMatch {
    pub name: String,
    pub start: usize,
    pub end: usize,
}

pub fn parse_wiki_links(_text: &str) -> Vec<WikiLinkMatch> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_link() {
        let text = "ここに [[foo]] があります";
        let got = parse_wiki_links(text);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "foo");
    }

    #[test]
    fn test_parse_link_with_md_extension() {
        let got = parse_wiki_links("[[bar.md]]");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "bar.md");
    }

    #[test]
    fn test_parse_multiple_links() {
        let got = parse_wiki_links("[[a]] と [[b]] と [[c]]");
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].name, "a");
        assert_eq!(got[1].name, "b");
        assert_eq!(got[2].name, "c");
    }

    #[test]
    fn test_parse_ignores_pipes_and_brackets() {
        // | を含むリンクは対象外
        assert_eq!(parse_wiki_links("[[foo|表示名]]").len(), 0);
        // 空リンクは対象外
        assert_eq!(parse_wiki_links("[[]]").len(), 0);
    }

    #[test]
    fn test_parse_returns_byte_offsets() {
        let text = "xx[[foo]]yy";
        let got = parse_wiki_links(text);
        assert_eq!(got[0].start, 2);
        assert_eq!(got[0].end, 9);
        assert_eq!(&text[got[0].start..got[0].end], "[[foo]]");
    }
}
```

`parser-core/src/lib.rs` の先頭付近に以下を追加：

```rust
pub mod wiki_link;
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `mise run test-rust`
Expected: `test_parse_single_link` 他が `unimplemented!` で panic して FAIL

- [ ] **Step 3: `parse_wiki_links` を実装**

`parser-core/src/wiki_link.rs` の `parse_wiki_links` を置き換え：

```rust
pub fn parse_wiki_links(text: &str) -> Vec<WikiLinkMatch> {
    let re = Regex::new(r"\[\[([^\[\]|]+?)\]\]").unwrap();
    re.captures_iter(text)
        .filter_map(|caps| {
            let whole = caps.get(0)?;
            let name = caps.get(1)?.as_str().to_string();
            if name.is_empty() {
                return None;
            }
            Some(WikiLinkMatch {
                name,
                start: whole.start(),
                end: whole.end(),
            })
        })
        .collect()
}
```

- [ ] **Step 4: テストがパスすることを確認**

Run: `mise run test-rust`
Expected: PASS（新規テスト 5 件すべて）

- [ ] **Step 5: コミット**

```bash
git add parser-core/src/wiki_link.rs parser-core/src/lib.rs
git commit -m "feat: parse_wiki_links で [[...]] 記法を抽出する"
```

---

## Task 2: リンク名の正規化（日付判定と .md 除去）

**Files:**
- Modify: `parser-core/src/wiki_link.rs`

- [ ] **Step 1: 失敗テストを追加**

`parser-core/src/wiki_link.rs` の `WikiLinkMatch` 定義の下に型を追加し、テストモジュール内にテストを追加：

```rust
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedName {
    pub name: String,
    pub is_journal: bool,
}

pub fn normalize_wiki_name(_raw: &str) -> NormalizedName {
    unimplemented!()
}
```

`mod tests` 内に追加：

```rust
    #[test]
    fn test_normalize_plain() {
        let got = normalize_wiki_name("foo");
        assert_eq!(got.name, "foo");
        assert!(!got.is_journal);
    }

    #[test]
    fn test_normalize_strips_md_extension() {
        let got = normalize_wiki_name("foo.md");
        assert_eq!(got.name, "foo");
        assert!(!got.is_journal);
    }

    #[test]
    fn test_normalize_detects_journal_date() {
        let got = normalize_wiki_name("2026-04-14");
        assert_eq!(got.name, "2026-04-14");
        assert!(got.is_journal);
    }

    #[test]
    fn test_normalize_trims_whitespace() {
        let got = normalize_wiki_name("  foo  ");
        assert_eq!(got.name, "foo");
    }
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `mise run test-rust`
Expected: `unimplemented!` で FAIL

- [ ] **Step 3: 実装**

`parser-core/src/wiki_link.rs` の `normalize_wiki_name` を置き換え：

```rust
pub fn normalize_wiki_name(raw: &str) -> NormalizedName {
    let trimmed = raw.trim();
    let without_ext = trimmed
        .strip_suffix(".md")
        .unwrap_or(trimmed)
        .to_string();

    let date_re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    let is_journal = date_re.is_match(&without_ext);

    NormalizedName {
        name: without_ext,
        is_journal,
    }
}
```

- [ ] **Step 4: テストがパスすることを確認**

Run: `mise run test-rust`
Expected: PASS

- [ ] **Step 5: コミット**

```bash
git add parser-core/src/wiki_link.rs
git commit -m "feat: normalize_wiki_name でリンク名を正規化し日付を判定する"
```

---

## Task 3: 候補一覧からの解決

**Files:**
- Modify: `parser-core/src/wiki_link.rs`

- [ ] **Step 1: 失敗テストを追加**

`parser-core/src/wiki_link.rs` に関数スタブを追加：

```rust
use std::path::{Path, PathBuf};

pub fn resolve_wiki_link(_name: &str, _candidates: &[PathBuf]) -> Option<PathBuf> {
    unimplemented!()
}
```

`mod tests` 内に追加：

```rust
    #[test]
    fn test_resolve_finds_first_match() {
        let candidates = vec![
            PathBuf::from("/home/u/taski/foo.md"),
            PathBuf::from("/home/u/work/foo.md"),
        ];
        let got = resolve_wiki_link("foo", &candidates);
        assert_eq!(got, Some(PathBuf::from("/home/u/taski/foo.md")));
    }

    #[test]
    fn test_resolve_matches_stem_ignoring_extension() {
        let candidates = vec![PathBuf::from("/a/foo.md")];
        assert_eq!(
            resolve_wiki_link("foo", &candidates),
            Some(PathBuf::from("/a/foo.md"))
        );
    }

    #[test]
    fn test_resolve_returns_none_when_absent() {
        let candidates = vec![PathBuf::from("/a/bar.md")];
        assert_eq!(resolve_wiki_link("foo", &candidates), None);
    }

    #[test]
    fn test_resolve_matches_journal_date() {
        let candidates = vec![PathBuf::from(
            "/home/u/taski/journal/2026/04/2026-04-14.md",
        )];
        assert_eq!(
            resolve_wiki_link("2026-04-14", &candidates),
            Some(PathBuf::from(
                "/home/u/taski/journal/2026/04/2026-04-14.md"
            ))
        );
    }
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `mise run test-rust`
Expected: FAIL

- [ ] **Step 3: 実装**

`parser-core/src/wiki_link.rs`：

```rust
pub fn resolve_wiki_link(name: &str, candidates: &[PathBuf]) -> Option<PathBuf> {
    let target = Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string());

    for candidate in candidates {
        let stem = candidate
            .file_stem()
            .map(|s| s.to_string_lossy().to_string());
        if stem.as_deref() == Some(target.as_str()) {
            return Some(candidate.clone());
        }
    }
    None
}
```

- [ ] **Step 4: テストがパスすることを確認**

Run: `mise run test-rust`
Expected: PASS

- [ ] **Step 5: コミット**

```bash
git add parser-core/src/wiki_link.rs
git commit -m "feat: resolve_wiki_link で候補から最初の一致を返す"
```

---

## Task 4: 作成先パスと初期内容の生成

**Files:**
- Modify: `parser-core/src/wiki_link.rs`

- [ ] **Step 1: 失敗テストを追加**

`parser-core/src/wiki_link.rs` に関数スタブ：

```rust
pub fn wiki_link_create_path(_name: &str, _is_journal: bool, _taski_home: &Path) -> PathBuf {
    unimplemented!()
}

pub fn wiki_link_initial_content(_name: &str) -> String {
    unimplemented!()
}
```

`mod tests` 内に：

```rust
    #[test]
    fn test_create_path_note() {
        let home = PathBuf::from("/home/u/taski");
        let got = wiki_link_create_path("foo", false, &home);
        assert_eq!(got, PathBuf::from("/home/u/taski/note/foo.md"));
    }

    #[test]
    fn test_create_path_journal() {
        let home = PathBuf::from("/home/u/taski");
        let got = wiki_link_create_path("2026-04-14", true, &home);
        assert_eq!(
            got,
            PathBuf::from("/home/u/taski/journal/2026/04/2026-04-14.md")
        );
    }

    #[test]
    fn test_initial_content() {
        assert_eq!(wiki_link_initial_content("foo"), "# foo\n");
    }
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `mise run test-rust`
Expected: FAIL

- [ ] **Step 3: 実装**

`parser-core/src/wiki_link.rs`：

```rust
pub fn wiki_link_create_path(name: &str, is_journal: bool, taski_home: &Path) -> PathBuf {
    if is_journal {
        // 期待形式: YYYY-MM-DD
        let year = &name[0..4];
        let month = &name[5..7];
        taski_home
            .join("journal")
            .join(year)
            .join(month)
            .join(format!("{name}.md"))
    } else {
        taski_home.join("note").join(format!("{name}.md"))
    }
}

pub fn wiki_link_initial_content(name: &str) -> String {
    format!("# {name}\n")
}
```

- [ ] **Step 4: テストがパスすることを確認**

Run: `mise run test-rust`
Expected: PASS

- [ ] **Step 5: コミット**

```bash
git add parser-core/src/wiki_link.rs
git commit -m "feat: wiki_link_create_path と初期内容生成を追加"
```

---

## Task 5: WASM への公開

**Files:**
- Modify: `parser-wasm/src/lib.rs`

- [ ] **Step 1: WASM 関数を追加**

`parser-wasm/src/lib.rs` の先頭 `pub use` に `wiki_link` 関連を追加：

```rust
pub use parser_core::wiki_link::{NormalizedName, WikiLinkMatch};
```

ファイル末尾に追加：

```rust
#[wasm_bindgen(js_name = "parseWikiLinks")]
pub fn parse_wiki_links(text: &str) -> JsValue {
    let links = parser_core::wiki_link::parse_wiki_links(text);
    serde_wasm_bindgen::to_value(&links).unwrap()
}

#[wasm_bindgen(js_name = "normalizeWikiName")]
pub fn normalize_wiki_name(raw: &str) -> JsValue {
    let normalized = parser_core::wiki_link::normalize_wiki_name(raw);
    serde_wasm_bindgen::to_value(&normalized).unwrap()
}

#[wasm_bindgen(js_name = "resolveWikiLink")]
pub fn resolve_wiki_link(name: &str, candidate_paths: Vec<String>) -> Option<String> {
    let candidates: Vec<std::path::PathBuf> =
        candidate_paths.into_iter().map(std::path::PathBuf::from).collect();
    parser_core::wiki_link::resolve_wiki_link(name, &candidates)
        .map(|p| p.to_string_lossy().to_string())
}

#[wasm_bindgen(js_name = "wikiLinkCreatePath")]
pub fn wiki_link_create_path(name: &str, is_journal: bool, taski_home: &str) -> String {
    parser_core::wiki_link::wiki_link_create_path(
        name,
        is_journal,
        std::path::Path::new(taski_home),
    )
    .to_string_lossy()
    .to_string()
}

#[wasm_bindgen(js_name = "wikiLinkInitialContent")]
pub fn wiki_link_initial_content(name: &str) -> String {
    parser_core::wiki_link::wiki_link_initial_content(name)
}
```

- [ ] **Step 2: WASM をビルド**

Run: `mise run build-wasm`
Expected: 成功、`src/pkg/` に更新されたファイルが生成される

- [ ] **Step 3: コミット**

```bash
git add parser-wasm/src/lib.rs
git commit -m "feat: wiki_link 関連関数を WASM に公開"
```

---

## Task 6: TypeScript 側の WASM ラッパ

**Files:**
- Modify: `src/parser.ts`

- [ ] **Step 1: 型と関数を追加**

`src/parser.ts` の import 文を拡張：

```typescript
import {
	parseTasks as wasmParseTasks,
	parseTasksAllDates as wasmParseTasksAllDates,
	buildTreeData as wasmBuildTreeData,
	buildScheduleData as wasmBuildScheduleData,
	extractTags as wasmExtractTags,
	extractFileTags as wasmExtractFileTags,
	parseWikiLinks as wasmParseWikiLinks,
	normalizeWikiName as wasmNormalizeWikiName,
	resolveWikiLink as wasmResolveWikiLink,
	wikiLinkCreatePath as wasmWikiLinkCreatePath,
	wikiLinkInitialContent as wasmWikiLinkInitialContent,
} from './pkg/parser_wasm';
```

ファイル末尾に追加：

```typescript
export interface WikiLinkMatch {
	name: string;
	start: number;
	end: number;
}

export interface NormalizedWikiName {
	name: string;
	isJournal: boolean;
}

export function parseWikiLinks(text: string): WikiLinkMatch[] {
	return wasmParseWikiLinks(text) as WikiLinkMatch[];
}

export function normalizeWikiName(raw: string): NormalizedWikiName {
	return wasmNormalizeWikiName(raw) as NormalizedWikiName;
}

export function resolveWikiLink(name: string, candidatePaths: string[]): string | undefined {
	const got = wasmResolveWikiLink(name, candidatePaths) as string | undefined;
	return got ?? undefined;
}

export function wikiLinkCreatePath(name: string, isJournal: boolean, taskiHome: string): string {
	return wasmWikiLinkCreatePath(name, isJournal, taskiHome);
}

export function wikiLinkInitialContent(name: string): string {
	return wasmWikiLinkInitialContent(name);
}
```

- [ ] **Step 2: 型チェックを通す**

Run: `mise run check`
Expected: PASS（型エラーなし）

- [ ] **Step 3: コミット**

```bash
git add src/parser.ts
git commit -m "feat: wiki_link 関連関数の TypeScript ラッパを追加"
```

---

## Task 7: DocumentLinkProvider と内部コマンド

**Files:**
- Create: `src/wikiLinkProviders.ts`

- [ ] **Step 1: プロバイダとコマンドハンドラを実装**

`src/wikiLinkProviders.ts` を新規作成：

```typescript
import * as vscode from 'vscode';
import * as path from 'path';
import * as os from 'os';
import {
	parseWikiLinks,
	normalizeWikiName,
	resolveWikiLink,
	wikiLinkCreatePath,
	wikiLinkInitialContent,
} from './parser';
import { findAllMarkdownUris } from './fileScanner';

interface OpenWikiLinkArgs {
	name: string;
	fromUri: string;
}

export class WikiLinkDocumentLinkProvider implements vscode.DocumentLinkProvider {
	provideDocumentLinks(document: vscode.TextDocument): vscode.DocumentLink[] {
		const text = document.getText();
		const matches = parseWikiLinks(text);
		const links: vscode.DocumentLink[] = [];
		for (const m of matches) {
			const startPos = document.positionAt(m.start);
			const endPos = document.positionAt(m.end);
			const range = new vscode.Range(startPos, endPos);
			const args: OpenWikiLinkArgs = { name: m.name, fromUri: document.uri.toString() };
			const target = vscode.Uri.parse(
				`command:taski.openWikiLink?${encodeURIComponent(JSON.stringify(args))}`
			);
			const link = new vscode.DocumentLink(range, target);
			link.tooltip = `Open [[${m.name}]]`;
			links.push(link);
		}
		return links;
	}
}

function rankCandidate(uri: vscode.Uri): number {
	const taskiHome = path.join(os.homedir(), 'taski');
	const fs = uri.fsPath;
	if (fs.startsWith(taskiHome + path.sep) || fs === taskiHome) {
		return 0;
	}
	const workspaceFolders = vscode.workspace.workspaceFolders ?? [];
	for (const folder of workspaceFolders) {
		const fp = folder.uri.fsPath;
		if (fs.startsWith(fp + path.sep) || fs === fp) {
			return 1;
		}
	}
	const config = vscode.workspace.getConfiguration('taski');
	const additionalDirs: string[] = config.get<string[]>('additionalDirectories', []);
	for (const dir of additionalDirs) {
		if (fs.startsWith(dir + path.sep) || fs === dir) {
			return 2;
		}
	}
	return 3;
}

export async function openWikiLink(args: OpenWikiLinkArgs): Promise<void> {
	const normalized = normalizeWikiName(args.name);
	const allUris = await findAllMarkdownUris();
	const sorted = [...allUris].sort((a, b) => rankCandidate(a) - rankCandidate(b));
	const candidatePaths = sorted.map((u) => u.fsPath);
	const matched = resolveWikiLink(normalized.name, candidatePaths);

	let targetPath: string;
	if (matched) {
		targetPath = matched;
	} else {
		const taskiHome = path.join(os.homedir(), 'taski');
		targetPath = wikiLinkCreatePath(normalized.name, normalized.isJournal, taskiHome);
		const targetUri = vscode.Uri.file(targetPath);
		try {
			await vscode.workspace.fs.stat(targetUri);
		} catch {
			const dir = path.dirname(targetPath);
			await vscode.workspace.fs.createDirectory(vscode.Uri.file(dir));
			const content = wikiLinkInitialContent(normalized.name);
			await vscode.workspace.fs.writeFile(targetUri, new TextEncoder().encode(content));
		}
	}

	const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(targetPath));
	await vscode.window.showTextDocument(doc);
}
```

- [ ] **Step 2: 型チェックを通す**

Run: `mise run check`
Expected: PASS

- [ ] **Step 3: コミット**

```bash
git add src/wikiLinkProviders.ts
git commit -m "feat: WikiLinkDocumentLinkProvider と openWikiLink コマンドを追加"
```

---

## Task 8: 拡張機能への登録

**Files:**
- Modify: `src/extension.ts`
- Modify: `package.json`

- [ ] **Step 1: `src/extension.ts` に登録処理を追加**

`src/extension.ts` の import に追加：

```typescript
import { WikiLinkDocumentLinkProvider, openWikiLink } from './wikiLinkProviders';
```

`activate` 関数の末尾（`openGitSyncOutputDisposable` の push の後）に追加：

```typescript
	// Wiki リンクプロバイダの登録
	const wikiLinkProviderDisposable = vscode.languages.registerDocumentLinkProvider(
		{ scheme: 'file', language: 'markdown' },
		new WikiLinkDocumentLinkProvider()
	);
	context.subscriptions.push(wikiLinkProviderDisposable);

	// Wiki リンクを開く内部コマンド
	const openWikiLinkDisposable = vscode.commands.registerCommand(
		'taski.openWikiLink',
		async (args: { name: string; fromUri: string }) => {
			await openWikiLink(args);
		}
	);
	context.subscriptions.push(openWikiLinkDisposable);
```

- [ ] **Step 2: `package.json` にコマンド定義を追加**

`package.json` の `contributes.commands` 配列の末尾（`taski.showSchedule` の後）にエントリを追加：

```json
      {
        "command": "taski.openWikiLink",
        "title": "Taski: Open Wiki Link (internal)"
      }
```

同じく `contributes.menus.commandPalette` がある場合、そこに `taski.openWikiLink` を非表示化するエントリを追加（無ければスキップ）。既存の `package.json` に `menus.commandPalette` 定義が無ければ、以下を `contributes` に追加：

```json
    "menus": {
      "commandPalette": [
        {
          "command": "taski.openWikiLink",
          "when": "false"
        }
      ]
    }
```

※ 既存の `contributes.menus` があればそこに merge すること。

- [ ] **Step 3: ビルドと型チェック**

Run: `mise run compile`
Expected: PASS

- [ ] **Step 4: コミット**

```bash
git add src/extension.ts package.json
git commit -m "feat: DocumentLinkProvider と openWikiLink コマンドを登録"
```

---

## Task 9: VSCode 統合テスト

**Files:**
- Create: `src/test/wikiLink.test.ts`

- [ ] **Step 1: テストを書く**

`src/test/wikiLink.test.ts` を新規作成：

```typescript
import * as assert from 'assert';
import * as path from 'path';
import * as os from 'os';
import * as fs from 'fs';
import * as vscode from 'vscode';
import { parseWikiLinks, normalizeWikiName, wikiLinkCreatePath } from '../parser';

suite('Wiki Link', () => {
	test('parseWikiLinks: 抽出できる', () => {
		const got = parseWikiLinks('ここに [[foo]] と [[bar.md]]');
		assert.strictEqual(got.length, 2);
		assert.strictEqual(got[0].name, 'foo');
		assert.strictEqual(got[1].name, 'bar.md');
	});

	test('normalizeWikiName: 日付を判定する', () => {
		const got = normalizeWikiName('2026-04-14');
		assert.strictEqual(got.name, '2026-04-14');
		assert.strictEqual(got.isJournal, true);
	});

	test('wikiLinkCreatePath: ノート向けのパスを組み立てる', () => {
		const taskiHome = path.join(os.tmpdir(), 'taski-test');
		const got = wikiLinkCreatePath('foo', false, taskiHome);
		assert.strictEqual(got, path.join(taskiHome, 'note', 'foo.md'));
	});

	test('wikiLinkCreatePath: ジャーナル向けのパスを組み立てる', () => {
		const taskiHome = path.join(os.tmpdir(), 'taski-test');
		const got = wikiLinkCreatePath('2026-04-14', true, taskiHome);
		assert.strictEqual(
			got,
			path.join(taskiHome, 'journal', '2026', '04', '2026-04-14.md')
		);
	});

	test('openWikiLink コマンド: 無いファイルを作成して開く', async () => {
		const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'taski-wiki-'));
		const originalHome = process.env.HOME;
		process.env.HOME = tmp;
		try {
			await vscode.commands.executeCommand('taski.openWikiLink', {
				name: 'newly-created',
				fromUri: vscode.Uri.file(path.join(tmp, 'dummy.md')).toString(),
			});
			const expected = path.join(tmp, 'taski', 'note', 'newly-created.md');
			assert.ok(fs.existsSync(expected), `ファイルが作成されているべき: ${expected}`);
			const content = fs.readFileSync(expected, 'utf8');
			assert.strictEqual(content, '# newly-created\n');
		} finally {
			process.env.HOME = originalHome;
			fs.rmSync(tmp, { recursive: true, force: true });
		}
	});
});
```

- [ ] **Step 2: テストを実行**

Run: `npm run test`
Expected: PASS

※ `HOME` の書き換えは `openWikiLink` 内の `os.homedir()` には即時には反映されない可能性があるため、テストが失敗する場合は該当テストを `test.skip` にするか、環境非依存な別のアサーション（例: `wikiLinkCreatePath` の直接テスト）に置き換えて対応する。前 4 件は必ず通ること。

- [ ] **Step 3: コミット**

```bash
git add src/test/wikiLink.test.ts
git commit -m "test: wiki link の smoke test を追加"
```

---

## Task 10: CLI サブコマンド `taski resolve`

**Files:**
- Modify: `cli/src/main.rs`

- [ ] **Step 1: `Resolve` バリアントを追加**

`cli/src/main.rs` の `enum Commands` に追加：

```rust
    /// [[...]] リンクの対応ファイルパスを解決。無ければ作成して出力
    Resolve {
        /// リンク名（[[...]] の中身と同等）
        name: String,

        /// 見つからなくても作成しない（exit 1）
        #[arg(long)]
        no_create: bool,

        /// 出力フォーマット（json）
        #[arg(long, short)]
        format: Option<String>,
    },
```

`match cli.command` に分岐を追加：

```rust
        Commands::Resolve { name, no_create, format } => {
            resolve_wiki(&name, no_create, format);
        }
```

そして関数本体を追加（`generate_agents_md` 関数の下など適切な位置）：

```rust
fn resolve_wiki(raw: &str, no_create: bool, format: Option<String>) {
    use parser_core::wiki_link::{
        normalize_wiki_name, resolve_wiki_link, wiki_link_create_path, wiki_link_initial_content,
    };

    let normalized = normalize_wiki_name(raw);
    let base_dir = taski_dir();
    let md_files = if base_dir.exists() {
        collect_md_files(&base_dir)
    } else {
        Vec::new()
    };

    let existing = resolve_wiki_link(&normalized.name, &md_files);

    let (path, created) = match existing {
        Some(p) => (p, false),
        None => {
            if no_create {
                eprintln!("エラー: {} に対応するファイルが見つかりません", raw);
                process::exit(1);
            }
            let target = wiki_link_create_path(&normalized.name, normalized.is_journal, &base_dir);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap_or_else(|e| {
                    eprintln!("エラー: ディレクトリを作成できません: {e}");
                    process::exit(1);
                });
            }
            if !target.exists() {
                let content = wiki_link_initial_content(&normalized.name);
                fs::write(&target, content).unwrap_or_else(|e| {
                    eprintln!("エラー: ファイルを作成できません: {e}");
                    process::exit(1);
                });
            }
            (target, true)
        }
    };

    match format.as_deref() {
        Some("json") => {
            let payload = serde_json::json!({
                "path": path.to_string_lossy(),
                "created": created,
                "is_journal": normalized.is_journal,
            });
            let json = serde_json::to_string_pretty(&payload).unwrap();
            println!("{json}");
        }
        Some(other) => {
            eprintln!("エラー: 未対応のフォーマットです: {other}");
            process::exit(1);
        }
        None => {
            println!("{}", path.display());
        }
    }
}
```

- [ ] **Step 2: 既存の Rust テストが壊れていないことを確認**

Run: `mise run test-rust`
Expected: PASS

- [ ] **Step 3: CLI をビルドして手動確認**

Run: `mise run build-cli`
Expected: 成功

手動確認：

```bash
# 存在しないリンクを作成して出力
./target/release/taski resolve test-wiki-link-foo
# 出力例: /Users/<you>/taski/note/test-wiki-link-foo.md

# JSON 出力
./target/release/taski resolve test-wiki-link-foo --format json
# 出力例: {"created": false, "is_journal": false, "path": "..."}

# --no-create で存在しないときはエラー
./target/release/taski resolve definitely-does-not-exist-xyz --no-create
# stderr にエラー、exit code 1
```

- [ ] **Step 4: コミット**

```bash
git add cli/src/main.rs
git commit -m "feat: taski resolve サブコマンドを追加"
```

---

## Task 11: ドキュメント更新

**Files:**
- Modify: `CLAUDE.md`
- Modify: `cli/AGENTS.md`(存在する場合)

- [ ] **Step 1: `CLAUDE.md` にコマンドと CLI を追記**

`CLAUDE.md` の「Architecture」節、ユーザー向けコマンド列挙箇所に `taski.openWikiLink` を内部コマンドとして追記する。

CLI サブコマンド節（`### Subcommands`）に追加：

```markdown
- `taski resolve <name>` — `[[name]]` の対応ファイルパスを出力。無ければ `$HOME/taski/note/<name>.md`（日付なら `$HOME/taski/journal/<YYYY>/<MM>/<name>.md`）を作成して出力。`--no-create` で作成抑止、`--format json` で構造化出力。
```

必要に応じて新しい節「Wiki リンクナビゲーション」を追加：

```markdown
### Wiki リンクナビゲーション

Markdown 内の `[[foo]]` を Cmd+Click すると、優先順位（`$HOME/taski` > workspace > 追加ディレクトリ > 開いているドキュメント）で既存 `foo.md` を探して開く。見つからなければ `$HOME/taski/note/foo.md` を `# foo\n` で作成して開く。`[[YYYY-MM-DD]]` はジャーナル `$HOME/taski/journal/<YYYY>/<MM>/<YYYY-MM-DD>.md` として扱う。
```

`cli/AGENTS.md` が存在する場合は、`taski resolve` の項目を同様に追記する。

- [ ] **Step 2: コミット**

```bash
git add CLAUDE.md cli/AGENTS.md 2>/dev/null || git add CLAUDE.md
git commit -m "docs: wiki リンクナビゲーション機能をドキュメント化"
```

---

## 完了条件

- [ ] `mise run test-rust` が PASS
- [ ] `mise run check` が PASS
- [ ] `npm run test` が PASS
- [ ] `mise run package` が PASS
- [ ] VSCode 上で `[[foo]]` を Cmd+Click → 既存 foo.md が開く、または `$HOME/taski/note/foo.md` が作成されて開く
- [ ] VSCode 上で `[[2026-04-14]]` を Cmd+Click → `$HOME/taski/journal/2026/04/2026-04-14.md` が作成/参照される
- [ ] `taski resolve foo` が期待パスを stdout に返す
- [ ] `taski resolve foo --no-create` が未存在時に exit code 1
