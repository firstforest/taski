import * as vscode from 'vscode';
import * as path from 'path';

export interface ParsedTask {
	isCompleted: boolean;
	text: string;
	line: number;
	log: string;
}

export function parseTasks(lines: string[], targetDate: string): ParsedTask[] {
	const tasks: ParsedTask[] = [];
	let currentTask: { indent: number; completed: boolean; text: string; line: number } | null = null;

	for (let i = 0; i < lines.length; i++) {
		const text = lines[i];

		const taskMatch = text.match(/^(\s*)-\s*\[([ x])\]\s*(.*)/);
		if (taskMatch) {
			currentTask = {
				indent: taskMatch[1].length,
				completed: taskMatch[2] === 'x',
				text: taskMatch[3],
				line: i
			};
			continue;
		}

		const dateMatch = text.match(/^(\s*)-\s*(\d{4}-\d{2}-\d{2}):\s*(.*)/);
		if (dateMatch && currentTask) {
			const dateIndent = dateMatch[1].length;
			const dateStr = dateMatch[2];
			const logContent = dateMatch[3];

			if (dateStr === targetDate && dateIndent > currentTask.indent) {
				tasks.push({
					isCompleted: currentTask.completed,
					text: currentTask.text,
					line: currentTask.line,
					log: logContent
				});
			}
		}
	}
	return tasks;
}

export function activate(context: vscode.ExtensionContext) {

	const myScheme = 'daily-tasks';
	const myProvider = new TodaysTaskProvider();

	context.subscriptions.push(vscode.workspace.registerTextDocumentContentProvider(myScheme, myProvider));

	let disposable = vscode.commands.registerCommand('daily-task-logger.showToday', async () => {
		// 【修正】ローカル時間の日付を取得
		const today = getLocalDateString();
		const uri = vscode.Uri.parse(`${myScheme}:summary/${today}.md`);

		// コンテンツの再取得を通知してからドキュメントを開く
		myProvider.refresh(uri);
		await vscode.workspace.openTextDocument(uri);
		await vscode.commands.executeCommand('markdown.showPreviewToSide', uri);
	});

	context.subscriptions.push(disposable);
}

// 【追加】ローカルタイムゾーンで YYYY-MM-DD を取得する関数
function getLocalDateString(): string {
	const d = new Date();
	const year = d.getFullYear();
	const month = String(d.getMonth() + 1).padStart(2, '0');
	const day = String(d.getDate()).padStart(2, '0');
	return `${year}-${month}-${day}`;
}

class TodaysTaskProvider implements vscode.TextDocumentContentProvider {

	private _onDidChange = new vscode.EventEmitter<vscode.Uri>();
	readonly onDidChange = this._onDidChange.event;

	refresh(uri: vscode.Uri): void {
		this._onDidChange.fire(uri);
	}

	async provideTextDocumentContent(uri: vscode.Uri): Promise<string> {
		// 【修正】ローカル時間の日付を取得
		const todayStr = getLocalDateString();

		const workspaceFiles = await vscode.workspace.findFiles('**/*.md', '**/node_modules/**');

		// ワークスペース内のファイル + 開いている .md ファイルを合算し、URI で重複排除
		const seen = new Set<string>();
		const allFileUris: vscode.Uri[] = [];
		for (const uri of workspaceFiles) {
			const key = uri.toString();
			if (!seen.has(key)) {
				seen.add(key);
				allFileUris.push(uri);
			}
		}
		for (const doc of vscode.workspace.textDocuments) {
			if (doc.uri.scheme === 'file' && doc.languageId === 'markdown') {
				const key = doc.uri.toString();
				if (!seen.has(key)) {
					seen.add(key);
					allFileUris.push(doc.uri);
				}
			}
		}

		let outputMarkdown = `# 今日のタスク一覧 (${todayStr})\n\n`;
		let hasTasks = false;

		for (const fileUri of allFileUris) {
			if (fileUri.scheme === 'daily-tasks') { continue; }

			const doc = await vscode.workspace.openTextDocument(fileUri);
			const tasksInFile = this.extractTasks(doc, todayStr);

			if (tasksInFile.length > 0) {
				hasTasks = true;
				const relativePath = vscode.workspace.asRelativePath(fileUri);
				outputMarkdown += `## ${path.basename(relativePath)}\n`;

				for (const task of tasksInFile) {
					// ジャンプ用のリンクを作成
					outputMarkdown += `- [${task.isCompleted ? 'x' : ' '}] [${task.text}](${fileUri.path}#L${task.line})\n`;
					// ログ部分
					outputMarkdown += `    - 📝 ${task.log}\n`;
				}
				outputMarkdown += `\n`;
			}
		}

		if (!hasTasks) {
			outputMarkdown += `今日のタスク（ログ行: ${todayStr}）は見つかりませんでした。\n`;
			outputMarkdown += `タスクの下に "- ${todayStr}: ログ" を追加してみてください。\n`;
			outputMarkdown += `※Markdownファイルが保存されているかも確認してください。`;
		}

		return outputMarkdown;
	}

	private extractTasks(doc: vscode.TextDocument, targetDate: string): ParsedTask[] {
		const lines: string[] = [];
		for (let i = 0; i < doc.lineCount; i++) {
			lines.push(doc.lineAt(i).text);
		}
		return parseTasks(lines, targetDate);
	}
}
