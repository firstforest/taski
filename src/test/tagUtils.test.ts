import * as assert from 'assert';
import { extractTags, extractFileTags } from '../tagUtils';

suite('extractTags', () => {

	test('単一タグを抽出する', () => {
		const result = extractTags('タスク #frontend');
		assert.deepStrictEqual(result, ['frontend']);
	});

	test('複数タグを抽出する', () => {
		const result = extractTags('ログイン画面を修正 #frontend #bug');
		assert.deepStrictEqual(result, ['frontend', 'bug']);
	});

	test('タグなしの場合は空配列を返す', () => {
		const result = extractTags('タグなしのタスク');
		assert.deepStrictEqual(result, []);
	});

	test('日本語タグを抽出する', () => {
		const result = extractTags('タスク #フロントエンド #バグ修正');
		assert.deepStrictEqual(result, ['フロントエンド', 'バグ修正']);
	});

	test('空文字列は空配列を返す', () => {
		const result = extractTags('');
		assert.deepStrictEqual(result, []);
	});

	test('#のみの場合は空配列を返す', () => {
		const result = extractTags('#');
		assert.deepStrictEqual(result, []);
	});

	test('タスクチェックボックスと混在する場合', () => {
		const result = extractTags('ログイン画面を修正 #frontend #bug');
		assert.deepStrictEqual(result, ['frontend', 'bug']);
	});

	test('連続呼び出しでも正しく動作する', () => {
		const result1 = extractTags('タスク1 #tag1');
		const result2 = extractTags('タスク2 #tag2 #tag3');
		assert.deepStrictEqual(result1, ['tag1']);
		assert.deepStrictEqual(result2, ['tag2', 'tag3']);
	});
});

suite('extractFileTags', () => {

	test('project: true の場合はファイル名をタグにする', () => {
		const result = extractFileTags(
			['---', 'project: true', '---', '- [ ] タスク'],
			'projectA.md',
		);
		assert.deepStrictEqual(result, ['projectA']);
	});

	test('ファイル名のスペースは _ に置換する', () => {
		const result = extractFileTags(
			['---', 'project: true', '---'],
			'2026-04-14 会議メモ.md',
		);
		assert.deepStrictEqual(result, ['2026-04-14_会議メモ']);
	});

	test('project: false の場合は空配列を返す', () => {
		const result = extractFileTags(
			['---', 'project: false', '---'],
			'projectA.md',
		);
		assert.deepStrictEqual(result, []);
	});

	test('front matter がない場合は空配列を返す', () => {
		const result = extractFileTags(['- [ ] タスク'], 'projectA.md');
		assert.deepStrictEqual(result, []);
	});

	test('旧仕様の tags フィールドは無視される', () => {
		const result = extractFileTags(
			['---', 'tags:', '  - old', '---'],
			'projectA.md',
		);
		assert.deepStrictEqual(result, []);
	});
});
