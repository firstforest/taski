import * as assert from 'assert';
import { parseTasks, parseTasksAllDates } from '../extension';

suite('parseTasks', () => {

	test('基本: 未完了タスクとログを抽出する', () => {
		const lines = [
			'- [ ] タスクA',
			'    - 2026-02-01: ログA',
		];
		const result = parseTasks(lines, '2026-02-01');
		assert.strictEqual(result.length, 1);
		assert.strictEqual(result[0].isCompleted, false);
		assert.strictEqual(result[0].text, 'タスクA');
		assert.strictEqual(result[0].log, 'ログA');
		assert.strictEqual(result[0].line, 0);
	});

	test('基本: 完了タスクとログを抽出する', () => {
		const lines = [
			'- [x] 完了タスク',
			'    - 2026-02-01: 完了ログ',
		];
		const result = parseTasks(lines, '2026-02-01');
		assert.strictEqual(result.length, 1);
		assert.strictEqual(result[0].isCompleted, true);
		assert.strictEqual(result[0].text, '完了タスク');
		assert.strictEqual(result[0].log, '完了ログ');
	});

	test('対象日付以外のログは含まれない', () => {
		const lines = [
			'- [ ] タスク',
			'    - 2026-01-31: 昨日のログ',
			'    - 2026-02-01: 今日のログ',
			'    - 2026-02-02: 明日のログ',
		];
		const result = parseTasks(lines, '2026-02-01');
		assert.strictEqual(result.length, 1);
		assert.strictEqual(result[0].log, '今日のログ');
	});

	test('複数タスクを抽出する', () => {
		const lines = [
			'- [ ] タスク1',
			'    - 2026-02-01: ログ1',
			'- [x] タスク2',
			'    - 2026-02-01: ログ2',
		];
		const result = parseTasks(lines, '2026-02-01');
		assert.strictEqual(result.length, 2);
		assert.strictEqual(result[0].text, 'タスク1');
		assert.strictEqual(result[1].text, 'タスク2');
		assert.strictEqual(result[1].isCompleted, true);
	});

	test('同じタスクに同一日付のログが複数ある場合、すべて抽出する', () => {
		const lines = [
			'- [ ] タスク',
			'    - 2026-02-01: ログA',
			'    - 2026-02-01: ログB',
		];
		const result = parseTasks(lines, '2026-02-01');
		assert.strictEqual(result.length, 2);
		assert.strictEqual(result[0].log, 'ログA');
		assert.strictEqual(result[1].log, 'ログB');
	});

	test('ログ行のインデントがタスクより深くない場合は無視される', () => {
		const lines = [
			'    - [ ] タスク（インデント4）',
			'    - 2026-02-01: 同レベルのログ',
			'  - 2026-02-01: 浅いインデントのログ',
		];
		const result = parseTasks(lines, '2026-02-01');
		assert.strictEqual(result.length, 0);
	});

	test('タスクがない状態でログ行があっても無視される', () => {
		const lines = [
			'    - 2026-02-01: 孤立したログ',
		];
		const result = parseTasks(lines, '2026-02-01');
		assert.strictEqual(result.length, 0);
	});

	test('空の入力は空配列を返す', () => {
		const result = parseTasks([], '2026-02-01');
		assert.strictEqual(result.length, 0);
	});

	test('タスクはあるがログ行がない場合は空配列を返す', () => {
		const lines = [
			'- [ ] ログなしタスク',
		];
		const result = parseTasks(lines, '2026-02-01');
		assert.strictEqual(result.length, 0);
	});

	test('line はタスク行の0始まりの行番号を返す', () => {
		const lines = [
			'# ヘッダー',
			'',
			'- [ ] 3行目のタスク',
			'    - 2026-02-01: ログ',
		];
		const result = parseTasks(lines, '2026-02-01');
		assert.strictEqual(result.length, 1);
		assert.strictEqual(result[0].line, 2);
	});

	test('タスク間に無関係な行があっても正しく解析する', () => {
		const lines = [
			'- [ ] タスク1',
			'    - 2026-02-01: ログ1',
			'',
			'これは普通のテキスト',
			'',
			'- [x] タスク2',
			'    - 2026-02-01: ログ2',
		];
		const result = parseTasks(lines, '2026-02-01');
		assert.strictEqual(result.length, 2);
		assert.strictEqual(result[0].text, 'タスク1');
		assert.strictEqual(result[1].text, 'タスク2');
	});

	test('ネストされたタスクのログ行を正しく処理する', () => {
		const lines = [
			'- [ ] 親タスク',
			'    - [ ] 子タスク',
			'        - 2026-02-01: 子のログ',
		];
		const result = parseTasks(lines, '2026-02-01');
		// 子タスクが currentTask になり、そのログが抽出される
		assert.strictEqual(result.length, 1);
		assert.strictEqual(result[0].text, '子タスク');
		assert.strictEqual(result[0].log, '子のログ');
	});
});

suite('parseTasksAllDates', () => {

	test('全日付のタスクを抽出する', () => {
		const lines = [
			'- [ ] タスクA',
			'    - 2026-01-31: 昨日のログ',
			'    - 2026-02-01: 今日のログ',
			'    - 2026-02-02: 明日のログ',
		];
		const result = parseTasksAllDates(lines);
		assert.strictEqual(result.length, 3);
		assert.strictEqual(result[0].date, '2026-01-31');
		assert.strictEqual(result[0].log, '昨日のログ');
		assert.strictEqual(result[1].date, '2026-02-01');
		assert.strictEqual(result[1].log, '今日のログ');
		assert.strictEqual(result[2].date, '2026-02-02');
		assert.strictEqual(result[2].log, '明日のログ');
	});

	test('複数タスク・複数日付を抽出する', () => {
		const lines = [
			'- [ ] タスク1',
			'    - 2026-02-01: ログ1',
			'- [x] タスク2',
			'    - 2026-01-30: ログ2',
		];
		const result = parseTasksAllDates(lines);
		assert.strictEqual(result.length, 2);
		assert.strictEqual(result[0].text, 'タスク1');
		assert.strictEqual(result[0].date, '2026-02-01');
		assert.strictEqual(result[1].text, 'タスク2');
		assert.strictEqual(result[1].date, '2026-01-30');
	});

	test('インデントが浅いログ行は無視され、タスクは日付なしで返る', () => {
		const lines = [
			'    - [ ] タスク（インデント4）',
			'    - 2026-02-01: 同レベルのログ',
		];
		const result = parseTasksAllDates(lines);
		assert.strictEqual(result.length, 1);
		assert.strictEqual(result[0].text, 'タスク（インデント4）');
		assert.strictEqual(result[0].date, '');
		assert.strictEqual(result[0].log, '');
	});

	test('空の入力は空配列を返す', () => {
		const result = parseTasksAllDates([]);
		assert.strictEqual(result.length, 0);
	});

	test('ログ行がないタスクは日付なしで返る', () => {
		const lines = [
			'- [ ] ログなしタスク',
		];
		const result = parseTasksAllDates(lines);
		assert.strictEqual(result.length, 1);
		assert.strictEqual(result[0].text, 'ログなしタスク');
		assert.strictEqual(result[0].date, '');
		assert.strictEqual(result[0].log, '');
	});

	test('ログありとログなしのタスクが混在する場合', () => {
		const lines = [
			'- [ ] タスク1',
			'    - 2026-02-01: ログ1',
			'- [ ] タスク2',
			'- [x] タスク3',
			'    - 2026-01-30: ログ3',
		];
		const result = parseTasksAllDates(lines);
		assert.strictEqual(result.length, 3);
		assert.strictEqual(result[0].text, 'タスク1');
		assert.strictEqual(result[0].date, '2026-02-01');
		assert.strictEqual(result[1].text, 'タスク2');
		assert.strictEqual(result[1].date, '');
		assert.strictEqual(result[2].text, 'タスク3');
		assert.strictEqual(result[2].date, '2026-01-30');
	});
});
