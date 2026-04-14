#!/usr/bin/env node
// npm version のライフサイクルから呼ばれ、CHANGELOG.md に新バージョンのエントリを
// Claude Code（headless 実行）で生成して追記する。

const { execSync, spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const repoRoot = path.resolve(__dirname, '..');
const changelogPath = path.join(repoRoot, 'CHANGELOG.md');

const pkg = JSON.parse(fs.readFileSync(path.join(repoRoot, 'package.json'), 'utf8'));
const newVersion = pkg.version;

// 直近のタグ（無ければ全履歴）からのコミットログを取得
let range = '';
try {
    const lastTag = execSync('git describe --tags --abbrev=0', { cwd: repoRoot })
        .toString()
        .trim();
    range = `${lastTag}..HEAD`;
} catch {
    range = '';
}

const log = execSync(`git log ${range} --no-merges --pretty=format:%s%n%b%n---`, {
    cwd: repoRoot,
}).toString().trim();

if (!log) {
    console.log('[update-changelog] 新しいコミットがないためスキップします');
    process.exit(0);
}

const today = new Date();
const pad = (n) => String(n).padStart(2, '0');
const dateStr = `${today.getFullYear()}-${pad(today.getMonth() + 1)}-${pad(today.getDate())}`;

const prompt = `以下は前回リリース以降の git コミットログです。これを元に Keep a Changelog 形式の CHANGELOG エントリを日本語で作成してください。

要件:
- 出力は \`## [${newVersion}] - ${dateStr}\` から始めること
- 変更を Added / Changed / Fixed / Removed のいずれかに分類（該当がないセクションは省略）
- 箇条書きで簡潔に。ユーザー視点の変更のみ記載し、内部リファクタやビルド調整は省いて良い
- 余計な前置き・後書き・コードフェンス・説明文は一切出力せず、CHANGELOG に貼り付けられる Markdown 本文のみを出力
- 末尾に改行を含めること

コミットログ:
${log}
`;

console.log(`[update-changelog] ${newVersion} のエントリを Claude で生成中...`);

const result = spawnSync('claude', ['-p', prompt], {
    cwd: repoRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'inherit'],
});

if (result.status !== 0) {
    console.error('[update-changelog] claude の実行に失敗しました');
    process.exit(result.status ?? 1);
}

let entry = result.stdout.trim();
// 万一コードフェンスで囲まれていた場合は剥がす
entry = entry.replace(/^```(?:markdown)?\n?/i, '').replace(/\n?```$/i, '').trim();

if (!entry.startsWith(`## [${newVersion}]`)) {
    console.error('[update-changelog] 生成結果が想定形式と異なります。出力:\n' + entry);
    process.exit(1);
}

const existing = fs.readFileSync(changelogPath, 'utf8');
// ヘッダ（最初に現れる `## ` の直前）を境に分割して差し込む
const headerEnd = existing.search(/^## \[/m);
const head = headerEnd >= 0 ? existing.slice(0, headerEnd) : existing + '\n';
const tail = headerEnd >= 0 ? existing.slice(headerEnd) : '';

const next = `${head}${entry}\n\n${tail}`;
fs.writeFileSync(changelogPath, next);

execSync('git add CHANGELOG.md', { cwd: repoRoot });
console.log('[update-changelog] CHANGELOG.md を更新・ステージしました');
