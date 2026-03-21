import * as vscode from 'vscode';
import * as path from 'path';
import { findAllMarkdownUris } from './fileScanner';
import { buildScheduleData } from './parser';
import type { FileInput, ScheduleEntry } from './parser';

export class SchedulePanel {
	public static currentPanel: SchedulePanel | undefined;
	private static readonly viewType = 'taskiSchedule';

	private readonly panel: vscode.WebviewPanel;
	private disposables: vscode.Disposable[] = [];

	public static createOrShow(): void {
		if (SchedulePanel.currentPanel) {
			SchedulePanel.currentPanel.panel.reveal(vscode.ViewColumn.One);
			SchedulePanel.currentPanel.refresh();
			return;
		}

		const panel = vscode.window.createWebviewPanel(
			SchedulePanel.viewType,
			'スケジュール',
			vscode.ViewColumn.One,
			{ enableScripts: true }
		);

		SchedulePanel.currentPanel = new SchedulePanel(panel);
	}

	private constructor(panel: vscode.WebviewPanel) {
		this.panel = panel;

		this.panel.onDidDispose(() => this.dispose(), null, this.disposables);

		this.refresh();
	}

	public async refresh(): Promise<void> {
		const entries = await this.collectScheduleEntries();
		const todayStr = getLocalDateString();
		this.panel.webview.html = this.getHtmlForWebview(entries, todayStr);
	}

	public dispose(): void {
		SchedulePanel.currentPanel = undefined;
		this.panel.dispose();
		while (this.disposables.length) {
			const d = this.disposables.pop();
			if (d) {
				d.dispose();
			}
		}
	}

	private async collectScheduleEntries(): Promise<ScheduleEntry[]> {
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

		const todayStr = getLocalDateString();
		return buildScheduleData(files, todayStr);
	}

	private getHtmlForWebview(entries: ScheduleEntry[], todayStr: string): string {
		const nonce = getNonce();

		// 15分スロットを生成（6:00〜22:00）
		const slots: string[] = [];
		for (let h = 6; h <= 21; h++) {
			for (let m = 0; m < 60; m += 15) {
				slots.push(`${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`);
			}
		}

		// エントリを時刻スロットにマッピング
		const slotMap = new Map<string, ScheduleEntry[]>();
		const noTimeEntries: ScheduleEntry[] = [];

		for (const entry of entries) {
			if (entry.time === '') {
				noTimeEntries.push(entry);
				continue;
			}
			// 15分単位に切り捨て
			const roundedTime = roundToSlot(entry.time);
			if (!slotMap.has(roundedTime)) {
				slotMap.set(roundedTime, []);
			}
			slotMap.get(roundedTime)!.push(entry);
		}

		// 現在時刻のスロットを計算
		const now = new Date();
		const currentSlot = `${String(now.getHours()).padStart(2, '0')}:${String(Math.floor(now.getMinutes() / 15) * 15).padStart(2, '0')}`;

		// テーブル行を生成
		let tableRows = '';
		for (const slot of slots) {
			const slotEntries = slotMap.get(slot) || [];
			const isCurrent = slot === currentSlot;
			const rowClass = isCurrent ? ' class="current-slot"' : '';

			if (slotEntries.length === 0) {
				tableRows += `<tr${rowClass}>
					<td class="time-cell">${slot}</td>
					<td class="plan-cell"></td>
					<td class="actual-cell"></td>
				</tr>\n`;
			} else {
				for (let j = 0; j < slotEntries.length; j++) {
					const e = slotEntries[j];
					const completedClass = e.isCompleted ? ' completed' : '';
					tableRows += `<tr${rowClass}>
						${j === 0 ? `<td class="time-cell" rowspan="${slotEntries.length}">${slot}</td>` : ''}
						<td class="plan-cell${completedClass}">${escapeHtml(e.taskText)}</td>
						<td class="actual-cell">${escapeHtml(e.logText)}</td>
					</tr>\n`;
				}
			}
		}

		// 時刻なしエントリ
		let noTimeRows = '';
		if (noTimeEntries.length > 0) {
			for (let j = 0; j < noTimeEntries.length; j++) {
				const e = noTimeEntries[j];
				const completedClass = e.isCompleted ? ' completed' : '';
				noTimeRows += `<tr>
					${j === 0 ? `<td class="time-cell" rowspan="${noTimeEntries.length}">--:--</td>` : ''}
					<td class="plan-cell${completedClass}">${escapeHtml(e.taskText)}</td>
					<td class="actual-cell">${escapeHtml(e.logText)}</td>
				</tr>\n`;
			}
		}

		return `<!DOCTYPE html>
<html lang="ja">
<head>
	<meta charset="UTF-8">
	<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}';">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<title>スケジュール - ${todayStr}</title>
	<style nonce="${nonce}">
		body {
			font-family: var(--vscode-font-family);
			font-size: var(--vscode-font-size);
			color: var(--vscode-foreground);
			background-color: var(--vscode-editor-background);
			padding: 12px;
			margin: 0;
		}
		h2 {
			margin: 0 0 12px 0;
			font-size: 1.2em;
			color: var(--vscode-foreground);
		}
		table {
			width: 100%;
			border-collapse: collapse;
			table-layout: fixed;
		}
		th {
			background-color: var(--vscode-editor-inactiveSelectionBackground);
			color: var(--vscode-foreground);
			padding: 6px 8px;
			text-align: left;
			border-bottom: 2px solid var(--vscode-panel-border);
			position: sticky;
			top: 0;
			z-index: 1;
		}
		th:first-child {
			width: 60px;
		}
		td {
			padding: 4px 8px;
			border-bottom: 1px solid var(--vscode-panel-border);
			vertical-align: top;
		}
		.time-cell {
			color: var(--vscode-descriptionForeground);
			font-variant-numeric: tabular-nums;
			white-space: nowrap;
			border-right: 1px solid var(--vscode-panel-border);
		}
		.plan-cell {
			color: var(--vscode-foreground);
		}
		.plan-cell.completed {
			text-decoration: line-through;
			color: var(--vscode-descriptionForeground);
		}
		.actual-cell {
			color: var(--vscode-foreground);
		}
		tr.current-slot {
			background-color: var(--vscode-editor-selectionBackground);
		}
		tr:hover {
			background-color: var(--vscode-list-hoverBackground);
		}
		.no-time-section {
			margin-top: 16px;
		}
		.section-label {
			color: var(--vscode-descriptionForeground);
			font-size: 0.9em;
			margin: 16px 0 4px 0;
		}
	</style>
</head>
<body>
	<h2>${todayStr} スケジュール</h2>
	<table>
		<thead>
			<tr>
				<th>時間</th>
				<th>予定</th>
				<th>実績</th>
			</tr>
		</thead>
		<tbody>
			${tableRows}
			${noTimeRows}
		</tbody>
	</table>
</body>
</html>`;
	}
}

function getLocalDateString(): string {
	const d = new Date();
	const year = d.getFullYear();
	const month = String(d.getMonth() + 1).padStart(2, '0');
	const day = String(d.getDate()).padStart(2, '0');
	return `${year}-${month}-${day}`;
}

function getNonce(): string {
	let text = '';
	const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
	for (let i = 0; i < 32; i++) {
		text += possible.charAt(Math.floor(Math.random() * possible.length));
	}
	return text;
}

function roundToSlot(time: string): string {
	const [h, m] = time.split(':').map(Number);
	const roundedM = Math.floor(m / 15) * 15;
	return `${String(h).padStart(2, '0')}:${String(roundedM).padStart(2, '0')}`;
}

function escapeHtml(text: string): string {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;');
}
