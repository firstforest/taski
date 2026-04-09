import * as vscode from 'vscode';
import * as path from 'path';
import { findAllMarkdownUris } from './fileScanner';
import { buildScheduleData } from './parser';
import type { FileInput, ScheduleEntry } from './parser';

export class TaskAlertManager implements vscode.Disposable {
	private timer: ReturnType<typeof setInterval> | undefined;
	private alertedKeys = new Set<string>();
	private lastDateStr = '';

	start(): void {
		// 30秒ごとにチェック
		this.timer = setInterval(() => this.check(), 30 * 1000);
		// 起動直後にも一度チェック
		this.check();
	}

	dispose(): void {
		if (this.timer) {
			clearInterval(this.timer);
			this.timer = undefined;
		}
	}

	private async check(): Promise<void> {
		const enabled = vscode.workspace.getConfiguration('taski').get<boolean>('taskAlert', true);
		if (!enabled) {
			return;
		}

		const todayStr = getLocalDateString();

		// 日付が変わったらアラート済みセットをリセット
		if (todayStr !== this.lastDateStr) {
			this.alertedKeys.clear();
			this.lastDateStr = todayStr;
		}

		const now = new Date();
		const currentMinutes = now.getHours() * 60 + now.getMinutes();

		const leadMinutes = vscode.workspace.getConfiguration('taski').get<number>('taskAlertLeadMinutes', 1);

		let entries: ScheduleEntry[];
		try {
			entries = await this.collectScheduleEntries(todayStr);
		} catch {
			return;
		}

		for (const entry of entries) {
			if (entry.time === '' || entry.isCompleted) {
				continue;
			}

			const alertKey = `${entry.fileUri}:${entry.taskLine}:${entry.time}`;
			if (this.alertedKeys.has(alertKey)) {
				continue;
			}

			const [h, m] = entry.time.split(':').map(Number);
			const entryMinutes = h * 60 + m;
			const diff = entryMinutes - currentMinutes;

			// leadMinutes分前〜開始時刻の範囲でアラート
			if (diff >= 0 && diff <= leadMinutes) {
				this.alertedKeys.add(alertKey);
				const timeLabel = diff === 0 ? '開始時刻です' : `あと${diff}分`;
				const message = `${entry.time} ${entry.taskText}（${timeLabel}）`;
				const action = await vscode.window.showInformationMessage(
					message,
					'タスクを開く'
				);
				if (action === 'タスクを開く') {
					await vscode.commands.executeCommand('taski.openTaskLocation', entry.fileUri, entry.taskLine);
				}
			}
		}
	}

	private async collectScheduleEntries(todayStr: string): Promise<ScheduleEntry[]> {
		const allFileUris = await findAllMarkdownUris();
		const files: FileInput[] = [];

		for (const fileUri of allFileUris) {
			const doc = await vscode.workspace.openTextDocument(fileUri);
			const lines: string[] = [];
			for (let i = 0; i < doc.lineCount; i++) {
				lines.push(doc.lineAt(i).text);
			}
			const relativePath = vscode.workspace.asRelativePath(fileUri);
			const fileName = path.basename(relativePath);
			files.push({ fileName, fileUri: fileUri.toString(), lines });
		}

		return buildScheduleData(files, todayStr);
	}
}

function getLocalDateString(): string {
	const d = new Date();
	const year = d.getFullYear();
	const month = String(d.getMonth() + 1).padStart(2, '0');
	const day = String(d.getDate()).padStart(2, '0');
	return `${year}-${month}-${day}`;
}
