import { describe, expect, it } from 'vitest';
import { parseInline, parseNotes } from './release-notes';

describe('parseNotes', () => {
	it('reads headings, paragraphs and lists, split by blank lines', () => {
		const notes = [
			'The usage bar is new.',
			'',
			'## New',
			'',
			'### One bar',
			'',
			'Weekly first.',
			'',
			'## Changed',
			'',
			'- Limited accounts fade.',
			'- Placeholders look like rows.'
		].join('\n');

		expect(parseNotes(notes)).toEqual([
			{ kind: 'paragraph', lines: [[{ text: 'The usage bar is new.' }]] },
			{ kind: 'heading', level: 2, text: [{ text: 'New' }] },
			{ kind: 'heading', level: 3, text: [{ text: 'One bar' }] },
			{ kind: 'paragraph', lines: [[{ text: 'Weekly first.' }]] },
			{ kind: 'heading', level: 2, text: [{ text: 'Changed' }] },
			{
				kind: 'list',
				items: [[{ text: 'Limited accounts fade.' }], [{ text: 'Placeholders look like rows.' }]]
			}
		]);
	});

	it('keeps the line breaks of plain-text notes inside one paragraph', () => {
		expect(parseNotes('Fixed sign-in routing.\r\nUsage bars are faster.')).toEqual([
			{
				kind: 'paragraph',
				lines: [[{ text: 'Fixed sign-in routing.' }], [{ text: 'Usage bars are faster.' }]]
			}
		]);
	});

	it('caps deep headings at level 3', () => {
		expect(parseNotes('##### Deep')).toEqual([
			{ kind: 'heading', level: 3, text: [{ text: 'Deep' }] }
		]);
	});
});

describe('parseInline', () => {
	it('marks bold and code runs, and keeps only the text of a link', () => {
		expect(
			parseInline('Pick **Calendar**, see `latest.json` or [Releasing](../README.md).')
		).toEqual([
			{ text: 'Pick ' },
			{ text: 'Calendar', bold: true },
			{ text: ', see ' },
			{ text: 'latest.json', code: true },
			{ text: ' or Releasing.' }
		]);
	});

	it('leaves markup it does not know as written', () => {
		expect(parseInline('<img src=x onerror=alert(1)>')).toEqual([
			{ text: '<img src=x onerror=alert(1)>' }
		]);
	});
});
