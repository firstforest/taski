import * as vscode from 'vscode';
import * as path from 'path';
import * as os from 'os';

export function matchesExcludePattern(filePath: string, patterns: string[]): boolean {
	for (const pattern of patterns) {
		const regexStr = pattern
			.replace(/[.+^${}()|[\]\\]/g, '\\$&')
			.replace(/\*\*/g, '<<GLOBSTAR>>')
			.replace(/\*/g, '[^/]*')
			.replace(/<<GLOBSTAR>>/g, '.*');
		const regex = new RegExp(regexStr);
		if (regex.test(filePath)) {
			return true;
		}
	}
	return false;
}

export async function findMarkdownFilesInDirectory(dirUri: vscode.Uri, excludePatterns: string[] = []): Promise<vscode.Uri[]> {
	const results: vscode.Uri[] = [];
	const entries = await vscode.workspace.fs.readDirectory(dirUri);
	for (const [name, type] of entries) {
		const childUri = vscode.Uri.joinPath(dirUri, name);
		if (type === vscode.FileType.Directory) {
			if (name === 'node_modules') {
				continue;
			}
			if (excludePatterns.length > 0 && matchesExcludePattern(childUri.fsPath, excludePatterns)) {
				continue;
			}
			const nested = await findMarkdownFilesInDirectory(childUri, excludePatterns);
			results.push(...nested);
		} else if (type === vscode.FileType.File && name.endsWith('.md')) {
			if (excludePatterns.length > 0 && matchesExcludePattern(childUri.fsPath, excludePatterns)) {
				continue;
			}
			results.push(childUri);
		}
	}
	return results;
}

export async function findAllMarkdownUris(): Promise<vscode.Uri[]> {
	const config = vscode.workspace.getConfiguration('taski');
	const excludeDirs: string[] = config.get<string[]>('excludeDirectories', []);
	const includeWorkspace: boolean = config.get<boolean>('includeWorkspace', false);

	const seen = new Set<string>();
	const allFileUris: vscode.Uri[] = [];

	// ワークスペースのスキャン（設定で有効な場合のみ）
	if (includeWorkspace) {
		const excludePatterns = ['**/node_modules/**', ...excludeDirs];
		const excludeGlob = `{${excludePatterns.join(',')}}`;
		const workspaceFiles = await vscode.workspace.findFiles('**/*.md', excludeGlob);

		for (const uri of workspaceFiles) {
			const key = uri.toString();
			if (!seen.has(key)) {
				seen.add(key);
				allFileUris.push(uri);
			}
		}
	}

	// 開いているマークダウンドキュメントは常に含める
	for (const doc of vscode.workspace.textDocuments) {
		if (doc.uri.scheme === 'file' && doc.languageId === 'markdown') {
			const key = doc.uri.toString();
			if (!seen.has(key)) {
				seen.add(key);
				allFileUris.push(doc.uri);
			}
		}
	}

	const userAdditionalDirs: string[] = config.get<string[]>('additionalDirectories', []);
	const defaultTaskiDir = path.join(os.homedir(), 'taski');
	const additionalDirs = [defaultTaskiDir, ...userAdditionalDirs];
	for (const dirPath of additionalDirs) {
		const dirUri = vscode.Uri.file(dirPath);
		try {
			const mdFiles = await findMarkdownFilesInDirectory(dirUri, excludeDirs);
			for (const uri of mdFiles) {
				const key = uri.toString();
				if (!seen.has(key)) {
					seen.add(key);
					allFileUris.push(uri);
				}
			}
		} catch {
			// ディレクトリが存在しない場合などはスキップ
		}
	}

	return allFileUris;
}
