/**
 * Release notes arrive as Markdown in latest.json. The update dialog shows the few constructs the
 * notes use (headings, paragraphs, `- ` lists, **bold** and `code`) as plain data, never as HTML, so
 * nothing in a downloaded manifest can inject markup. Anything else stays as written.
 */

/** A run of text in a line, optionally **bold** or `code`. */
export type Inline = { text: string; bold?: boolean; code?: boolean };

export type NotesBlock =
	| { kind: 'heading'; level: 1 | 2 | 3; text: Inline[] }
	/** One paragraph; its lines keep their own line breaks, as plain-text notes expect. */
	| { kind: 'paragraph'; lines: Inline[][] }
	| { kind: 'list'; items: Inline[][] };

/** Splits a line into plain, bold and code runs; a link keeps only its text. */
export function parseInline(line: string): Inline[] {
	const text = line.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1');
	return text
		.split(/(\*\*[^*]+\*\*|`[^`]+`)/)
		.filter((part) => part !== '')
		.map((part) => {
			if (part.startsWith('**') && part.endsWith('**') && part.length > 4) {
				return { text: part.slice(2, -2), bold: true };
			}
			if (part.startsWith('`') && part.endsWith('`') && part.length > 2) {
				return { text: part.slice(1, -1), code: true };
			}
			return { text: part };
		});
}

/** Reads release notes into blocks; a blank line ends a paragraph or list. */
export function parseNotes(notes: string): NotesBlock[] {
	const blocks: NotesBlock[] = [];
	let open: NotesBlock | null = null;
	const close = () => {
		if (open) blocks.push(open);
		open = null;
	};

	for (const raw of notes.split(/\r?\n/)) {
		const line = raw.trimEnd();
		const heading = /^(#{1,6})\s+(.*)$/.exec(line);
		const item = /^\s*[-*]\s+(.*)$/.exec(line);
		if (line.trim() === '') {
			close();
		} else if (heading) {
			close();
			const level = Math.min(heading[1].length, 3) as 1 | 2 | 3;
			blocks.push({ kind: 'heading', level, text: parseInline(heading[2]) });
		} else if (item) {
			if (open?.kind !== 'list') {
				close();
				open = { kind: 'list', items: [] };
			}
			open.items.push(parseInline(item[1]));
		} else {
			if (open?.kind !== 'paragraph') {
				close();
				open = { kind: 'paragraph', lines: [] };
			}
			open.lines.push(parseInline(line.trim()));
		}
	}
	close();
	return blocks;
}
