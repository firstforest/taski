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

	// HOME 差し替えが os.homedir() に反映されないためスキップ
	test.skip('openWikiLink コマンド: 無いファイルを作成して開く', async () => {
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
