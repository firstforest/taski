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

		// デフォルト表示範囲は9:00〜18:00、エントリの時刻が範囲外なら拡張
		let minHour = 9;
		let maxHour = 18;
		for (const entry of entries) {
			const times = [entry.time, entry.endTime].filter(t => t !== '');
			for (const t of times) {
				const [h, m] = t.split(':').map(Number);
				if (h < minHour) {
					minHour = h;
				}
				const needHour = m > 0 ? h + 1 : h;
				if (needHour > maxHour) {
					maxHour = needHour;
				}
			}
		}

		// 15分スロットを生成（minHour:00 〜 maxHour-1:45）
		const slots: string[] = [];
		for (let h = minHour; h < maxHour; h++) {
			for (let m = 0; m < 60; m += 15) {
				slots.push(`${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`);
			}
		}

		// エントリを時刻スロットにマッピング
		const slotMap = new Map<string, ScheduleEntry[]>();
		const noTimeEntries: ScheduleEntry[] = [];
		// 帯スケジュール: 開始スロットから終了スロットまでの範囲を持つエントリ
		const bandEntries: { entry: ScheduleEntry; startSlot: string; endSlot: string; spanCount: number }[] = [];

		for (const entry of entries) {
			if (entry.time === '') {
				noTimeEntries.push(entry);
				continue;
			}
			if (entry.endTime !== '') {
				// 帯スケジュール
				const startSlot = roundToSlot(entry.time);
				const endSlot = roundToSlot(entry.endTime);
				const startIdx = slots.indexOf(startSlot);
				const endIdx = slots.indexOf(endSlot);
				const spanCount = startIdx >= 0 && endIdx > startIdx ? endIdx - startIdx : 1;
				bandEntries.push({ entry, startSlot, endSlot, spanCount });
				continue;
			}
			// 15分単位に切り捨て
			const roundedTime = roundToSlot(entry.time);
			if (!slotMap.has(roundedTime)) {
				slotMap.set(roundedTime, []);
			}
			slotMap.get(roundedTime)!.push(entry);
		}

		// 帯エントリがカバーするスロットを記録
		const bandSlotMap = new Map<string, { entry: ScheduleEntry; isStart: boolean; spanCount: number }[]>();
		for (const band of bandEntries) {
			const startIdx = slots.indexOf(band.startSlot);
			if (startIdx < 0) {
				continue;
			}
			for (let si = startIdx; si < startIdx + band.spanCount && si < slots.length; si++) {
				if (!bandSlotMap.has(slots[si])) {
					bandSlotMap.set(slots[si], []);
				}
				bandSlotMap.get(slots[si])!.push({
					entry: band.entry,
					isStart: si === startIdx,
					spanCount: band.spanCount,
				});
			}
		}

		// 現在時刻のスロットを計算
		const now = new Date();
		const currentSlot = `${String(now.getHours()).padStart(2, '0')}:${String(Math.floor(now.getMinutes() / 15) * 15).padStart(2, '0')}`;

		// テーブル行を生成
		let tableRows = '';
		for (let slotIdx = 0; slotIdx < slots.length; slotIdx++) {
			const slot = slots[slotIdx];
			const slotEntries = slotMap.get(slot) || [];
			const bandItems = bandSlotMap.get(slot) || [];
			const isCurrent = slot === currentSlot;
			const rowClass = isCurrent ? ' class="current-slot"' : '';

			// このスロットで開始する帯エントリ
			const bandStarts = bandItems.filter(b => b.isStart);
			// このスロットが帯の途中（開始でない）か
			const isBandContinuation = bandItems.length > 0 && bandStarts.length === 0;

			const totalEntries = slotEntries.length + bandStarts.length;

			if (totalEntries === 0 && !isBandContinuation) {
				tableRows += `<tr${rowClass}>
					<td class="time-cell">${slot}</td>
					<td class="plan-cell"></td>
					<td class="actual-cell"></td>
				</tr>\n`;
			} else if (totalEntries === 0 && isBandContinuation) {
				// 帯の途中スロット: 時間セルと実績セルのみ（予定セルは rowspan でカバー済み）
				tableRows += `<tr${rowClass}>
					<td class="time-cell">${slot}</td>
					<td class="actual-cell"></td>
				</tr>\n`;
			} else {
				let rowIdx = 0;
				// 帯エントリ（開始スロット）
				for (const band of bandStarts) {
					const e = band.entry;
					const completedClass = e.isCompleted ? ' completed' : '';
					tableRows += `<tr${rowClass}>
						${rowIdx === 0 ? `<td class="time-cell" rowspan="${totalEntries}">${slot}</td>` : ''}
						<td class="plan-cell band-cell${completedClass}" rowspan="${band.spanCount}">${escapeHtml(e.taskText)}${e.logText ? '<br><span class="band-log">' + escapeHtml(e.logText) + '</span>' : ''}</td>
						${rowIdx === 0 && !isBandContinuation ? '<td class="actual-cell"></td>' : ''}
					</tr>\n`;
					rowIdx++;
				}
				// 通常のエントリ
				for (const e of slotEntries) {
					const completedClass = e.isCompleted ? ' completed' : '';
					tableRows += `<tr${rowClass}>
						${rowIdx === 0 ? `<td class="time-cell" rowspan="${totalEntries}">${slot}</td>` : ''}
						<td class="plan-cell${completedClass}">${escapeHtml(e.taskText)}</td>
						<td class="actual-cell">${escapeHtml(e.logText)}</td>
					</tr>\n`;
					rowIdx++;
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
		.band-cell {
			background-color: var(--vscode-editor-selectionBackground);
			border-left: 3px solid var(--vscode-focusBorder);
			vertical-align: middle;
		}
		.band-log {
			color: var(--vscode-descriptionForeground);
			font-size: 0.9em;
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
