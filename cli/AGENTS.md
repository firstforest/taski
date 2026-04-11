# taski CLI

`$HOME/taski/` 配下のMarkdownファイルからタスクを管理するCLIツール。

## インストール

```bash
cargo install --path cli
```

ビルド済みバイナリは `target/release/taski` に出力される。

## データディレクトリ

すべてのコマンドは `$HOME/taski/` を基準ディレクトリとして動作する。ジャーナルファイルは `$HOME/taski/journal/<year>/<month>/<YYYY-MM-DD>.md` に保存される。

## タスクのMarkdownフォーマット

```markdown
- [ ] 未完了タスク #tag1
    - 2026-04-11: ログエントリ
- [x] 完了済みタスク #tag2
    - 2026-04-10: 作業ログ
```

- タスク行: `- [ ]`(未完了) または `- [x]`(完了)
- ログ行: タスクよりインデントが深い `- YYYY-MM-DD: テキスト`
- タグ: タスクテキスト内の `#タグ名` パターン（スペースや`#`を含まない文字列）

## コマンド

### `taski memo <text>`

今日のジャーナルファイルにタイムスタンプ付きメモを追記する。

```bash
# 基本的な使い方
taski memo 会議のメモ
# => "- 14:30: 会議のメモ" が追記される

# タイムスタンプなし
taski memo --no-timestamp 買い物リスト
# => "- 買い物リスト" が追記される

# パイプで入力
echo "パイプからの入力" | taski memo
```

**オプション:**
- `--no-timestamp` — 時刻プレフィックスを付けない
- テキスト引数を省略した場合、stdinから読み取る（パイプ入力時のみ）

### `taski list`

`$HOME/taski/` 内のすべてのMarkdownファイルからタスクを収集し、日付別にグループ化して表示する。

```bash
# デフォルト表示（色付きテキスト）
taski list

# JSON形式で出力
taski list --format json

# YAML形式で出力
taski list --format yaml

# 特定のタグでフィルタ
taski list --tag work

# タグフィルタとJSON出力の組み合わせ
taski list --tag work --format json
```

**オプション:**
- `-f, --format <FORMAT>` — 出力フォーマット（`json` または `yaml`）
- `-t, --tag <TAG>` — 指定タグを含むタスクのみ表示（`#` は不要、例: `--tag work`）

**表示ルール:**
- 今日の日付のタスクは完了・未完了の両方を表示
- それ以外の日付は未完了タスクのみ表示
- ログのないタスクは「日付なし」グループに表示

**JSON出力の構造:**

```json
[
  {
    "dateKey": "2026-04-11",
    "label": "今日 (2026-04-11) (2/5)",
    "isToday": true,
    "completedCount": 2,
    "totalCount": 5,
    "fileGroups": [
      {
        "fileName": "journal/2026/04/2026-04-11.md",
        "fileUri": "/Users/user/taski/journal/2026/04/2026-04-11.md",
        "tasks": [
          {
            "isCompleted": false,
            "text": "タスク名 #tag",
            "fileUri": "/Users/user/taski/journal/2026/04/2026-04-11.md",
            "line": 3,
            "log": "ログ内容",
            "date": "2026-04-11"
          }
        ]
      }
    ]
  }
]
```

### `taski journal`

今日のジャーナルファイルを `$EDITOR` で開く。ファイルが存在しない場合は自動作成する。

```bash
# エディタで開く
taski journal

# パスだけ表示（エディタを開かない）
taski journal --print
# => /Users/user/taski/journal/2026/04/2026-04-11.md
```

**オプション:**
- `--print` — ファイルパスを標準出力に表示するだけ（エディタを起動しない）

`$EDITOR` が未設定の場合はパス表示にフォールバックする。

### `taski toggle <file> <line>`

指定ファイルの指定行にあるタスクの完了状態を切り替える（`[ ]` ↔ `[x]`）。

```bash
# 3行目のタスクをトグル
taski toggle ~/taski/tasks.md 3
```

**引数:**
- `<file>` — 対象Markdownファイルのパス
- `<line>` — 行番号（1始まり）

`list --format json` の出力に含まれる `fileUri` と `line` をそのまま使える。

## 終了コード

- `0` — 成功
- `1` — エラー（メッセージはstderrに出力）

## 典型的なワークフロー

```bash
# 今日のタスクを確認
taski list

# 特定プロジェクトのタスクだけ確認
taski list --tag myproject

# メモを追記
taski memo MTGで決まったこと: デプロイは来週

# タスクを完了にする
taski toggle ~/taski/journal/2026/04/2026-04-11.md 5

# 他のツールと連携（JSON出力をjqで加工）
taski list --format json | jq '.[].fileGroups[].tasks[] | select(.isCompleted == false) | .text'
```
