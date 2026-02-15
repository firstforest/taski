import * as vscode from 'vscode';
import * as path from 'path';
import * as os from 'os';
import { execFile } from 'child_process';
import * as fs from 'fs';

type SyncState = 'idle' | 'syncing' | 'conflict' | 'disabled';

interface GitError extends Error {
	stdout: string;
	stderr: string;
}

export class GitSyncManager implements vscode.Disposable {
	private timer: ReturnType<typeof setInterval> | undefined;
	private statusBarItem: vscode.StatusBarItem;
	private state: SyncState = 'idle';
	private taskiDir: string;
	private isSyncing = false;
	private outputChannel: vscode.OutputChannel;
	private saveDebounceTimer: ReturnType<typeof setTimeout> | undefined;
	private configChangeDisposable: vscode.Disposable | undefined;
	private saveDisposable: vscode.Disposable | undefined;

	constructor() {
		this.taskiDir = path.join(os.homedir(), 'taski');
		this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 0);
		this.statusBarItem.command = 'taski.syncNow';
		this.outputChannel = vscode.window.createOutputChannel('Taski Git Sync');

		// 設定変更の監視
		this.configChangeDisposable = vscode.workspace.onDidChangeConfiguration(e => {
			if (e.affectsConfiguration('taski.gitAutoSync')) {
				const config = vscode.workspace.getConfiguration('taski');
				const enabled = config.get<boolean>('gitAutoSync', true);
				if (enabled) {
					this.start();
				} else {
					this.stop();
				}
			} else if (e.affectsConfiguration('taski.gitSyncInterval')) {
				const config = vscode.workspace.getConfiguration('taski');
				const enabled = config.get<boolean>('gitAutoSync', true);
				if (enabled) {
					this.restartTimer();
				}
			}
		});

		// ファイル保存の監視
		this.saveDisposable = vscode.workspace.onDidSaveTextDocument(doc => {
			if (doc.uri.fsPath.startsWith(this.taskiDir) && doc.languageId === 'markdown') {
				this.debouncedSync();
			}
		});
	}

	dispose(): void {
		this.stop();
		this.statusBarItem.dispose();
		this.outputChannel.dispose();
		this.configChangeDisposable?.dispose();
		this.saveDisposable?.dispose();
		if (this.saveDebounceTimer) {
			clearTimeout(this.saveDebounceTimer);
		}
	}

	async start(): Promise<void> {
		const config = vscode.workspace.getConfiguration('taski');
		const enabled = config.get<boolean>('gitAutoSync', true);

		if (!enabled) {
			this.state = 'disabled';
			this.statusBarItem.hide();
			return;
		}

		// taskiディレクトリの存在確認
		if (!fs.existsSync(this.taskiDir)) {
			this.outputChannel.appendLine(`タスクディレクトリが存在しません: ${this.taskiDir}`);
			this.state = 'disabled';
			this.statusBarItem.hide();
			return;
		}

		// gitリポジトリの確認
		const isRepo = await this.isGitRepo();
		if (!isRepo) {
			this.outputChannel.appendLine(`${this.taskiDir} はGitリポジトリではありません。自動同期をスキップします。`);
			this.state = 'disabled';
			this.statusBarItem.hide();
			return;
		}

		this.outputChannel.appendLine('Git自動同期を開始します。');
		this.restartTimer();
		this.statusBarItem.show();

		// 初回同期
		await this.sync();
	}

	stop(): void {
		if (this.timer) {
			clearInterval(this.timer);
			this.timer = undefined;
		}
		this.state = 'disabled';
		this.statusBarItem.hide();
	}

	async syncNow(): Promise<void> {
		if (this.state === 'conflict') {
			// コンフリクト状態の時はOutputChannelを開く
			this.outputChannel.show();
			return;
		}
		await this.sync();
	}

	private async isGitRepo(): Promise<boolean> {
		try {
			const gitDir = path.join(this.taskiDir, '.git');
			return fs.existsSync(gitDir);
		} catch {
			return false;
		}
	}

	private async hasChanges(): Promise<boolean> {
		try {
			const output = await this.execGit('status', '--porcelain');
			return output.trim().length > 0;
		} catch {
			return false;
		}
	}

	private async sync(): Promise<void> {
		// 同時実行防止
		if (this.isSyncing) {
			return;
		}

		this.isSyncing = true;
		this.state = 'syncing';
		this.updateStatusBar();

		try {
			// 変更があるかチェック
			const hasChanges = await this.hasChanges();

			if (hasChanges) {
				this.outputChannel.appendLine('変更を検出しました。コミットします...');
				await this.execGit('add', '-A');

				const now = new Date();
				const dateStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
				const timeStr = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
				const commitMessage = `taski: 自動同期 ${dateStr} ${timeStr}`;

				await this.execGit('commit', '-m', commitMessage);
				this.outputChannel.appendLine(`コミット完了: ${commitMessage}`);
			}

			// Pull rebase
			this.outputChannel.appendLine('リモートから変更を取得します...');
			await this.execGit('pull', '--rebase');
			this.outputChannel.appendLine('Pull完了');

			// Push
			this.outputChannel.appendLine('リモートにプッシュします...');
			await this.execGit('push');
			this.outputChannel.appendLine('Push完了。同期が完了しました。');

			this.state = 'idle';
		} catch (error) {
			await this.handleSyncError(error as GitError);
		} finally {
			this.isSyncing = false;
			this.updateStatusBar();
		}
	}

	private async handleSyncError(error: GitError): Promise<void> {
		const errorMessage = error.message + '\n' + error.stderr;

		// コンフリクト検出
		if (errorMessage.includes('CONFLICT') || errorMessage.includes('could not apply') || errorMessage.includes('Failed to merge')) {
			this.outputChannel.appendLine('コンフリクトを検出しました。');
			this.outputChannel.appendLine(errorMessage);

			// Rebaseを中断して作業ツリーを復元
			try {
				await this.execGit('rebase', '--abort');
				this.outputChannel.appendLine('Rebaseを中断しました。');
			} catch {
				// abort失敗は無視
			}

			this.state = 'conflict';
			this.stop(); // タイマーを停止

			// ユーザーに通知
			const action = await vscode.window.showWarningMessage(
				'Taski: Git同期でコンフリクトが発生しました。手動で解決してください。',
				'ターミナルを開く',
				'再試行'
			);

			if (action === 'ターミナルを開く') {
				const terminal = vscode.window.createTerminal({
					name: 'Taski Git',
					cwd: this.taskiDir
				});
				terminal.show();
			} else if (action === '再試行') {
				// 再試行
				this.state = 'idle';
				await this.start();
			}
		} else if (errorMessage.includes('Could not resolve host') || errorMessage.includes('unable to access')) {
			// ネットワークエラー - ログのみで次回リトライ
			this.outputChannel.appendLine('ネットワークエラー。次回リトライします。');
			this.outputChannel.appendLine(errorMessage);
			this.state = 'idle';
		} else if (errorMessage.includes('git: command not found') || errorMessage.includes('not found')) {
			// gitがインストールされていない
			this.outputChannel.appendLine('Gitが見つかりません。Git自動同期を無効化します。');
			this.outputChannel.appendLine(errorMessage);
			this.state = 'disabled';
			this.stop();
		} else {
			// その他のエラー
			this.outputChannel.appendLine('同期エラー:');
			this.outputChannel.appendLine(errorMessage);
			this.state = 'idle';
		}
	}

	private async execGit(...args: string[]): Promise<string> {
		return new Promise((resolve, reject) => {
			execFile('git', args, { cwd: this.taskiDir }, (error, stdout, stderr) => {
				if (error) {
					const gitError = error as GitError;
					gitError.stdout = stdout;
					gitError.stderr = stderr;
					reject(gitError);
				} else {
					resolve(stdout);
				}
			});
		});
	}

	private updateStatusBar(): void {
		switch (this.state) {
			case 'idle':
				this.statusBarItem.text = '$(cloud) Taski';
				this.statusBarItem.tooltip = '同期済み';
				this.statusBarItem.backgroundColor = undefined;
				break;
			case 'syncing':
				this.statusBarItem.text = '$(sync~spin) Taski: 同期中...';
				this.statusBarItem.tooltip = '同期中...';
				this.statusBarItem.backgroundColor = undefined;
				break;
			case 'conflict':
				this.statusBarItem.text = '$(warning) Taski: コンフリクト';
				this.statusBarItem.tooltip = 'コンフリクトが発生しました。クリックして詳細を確認';
				this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
				this.statusBarItem.command = 'taski.openGitSyncOutput';
				break;
			case 'disabled':
				this.statusBarItem.hide();
				break;
		}
	}

	private restartTimer(): void {
		if (this.timer) {
			clearInterval(this.timer);
		}

		const config = vscode.workspace.getConfiguration('taski');
		const interval = config.get<number>('gitSyncInterval', 30);
		const intervalMs = Math.max(interval, 30) * 1000; // 最小30秒

		this.timer = setInterval(() => {
			this.sync();
		}, intervalMs);

		this.outputChannel.appendLine(`同期タイマーを設定しました（間隔: ${interval}秒）`);
	}

	private debouncedSync(): void {
		if (this.saveDebounceTimer) {
			clearTimeout(this.saveDebounceTimer);
		}
		this.saveDebounceTimer = setTimeout(() => {
			this.outputChannel.appendLine('ファイル保存を検出。同期を実行します...');
			this.sync();
		}, 10000); // 10秒のデバウンス
	}
}
