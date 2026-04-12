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

	test('front matter のタグを抽出する', () => {
		const result = extractFileTags([
			'---',
			'tags:',
			'  - projectA',
			'  - work',
			'---',
			'- [ ] タスク',
		]);
		assert.deepStrictEqual(result, ['projectA', 'work']);
	});

	test('front matter がない場合は空配列を返す', () => {
		const result = extractFileTags(['- [ ] タスク']);
		assert.deepStrictEqual(result, []);
	});

	test('#プレフィクスを除去する', () => {
		const result = extractFileTags(['---', 'tags:', '  - "#projectA"', '---']);
		assert.deepStrictEqual(result, ['projectA']);
	});

	test('空要素をスキップする', () => {
		const result = extractFileTags([
			'---',
			'tags:',
			'  - projectA',
			'  - ""',
			'---',
		]);
		assert.deepStrictEqual(result, ['projectA']);
	});

	test('tagsが配列でない場合は空配列を返す', () => {
		const result = extractFileTags(['---', 'tags: projectA', '---']);
		assert.deepStrictEqual(result, []);
	});
});
