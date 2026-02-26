const tagRegex = /#([^\s#]+)/g;

export function extractTags(text: string): string[] {
	const tags: string[] = [];
	let match;
	while ((match = tagRegex.exec(text)) !== null) {
		tags.push(match[1]);
	}
	tagRegex.lastIndex = 0;
	return tags;
}
