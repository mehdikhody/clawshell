import { describe, expect, it } from 'vitest';
import { moveAccount } from './account-order';

describe('moveAccount', () => {
	const ids = ['a', 'b', 'c', 'd'];

	it('moves a row up to just before the target', () => {
		expect(moveAccount(ids, 'd', 'b', 'before')).toEqual(['a', 'd', 'b', 'c']);
	});

	it('moves a row down to just after the target', () => {
		expect(moveAccount(ids, 'a', 'c', 'after')).toEqual(['b', 'c', 'a', 'd']);
	});

	it('moves a row to the very end and the very start', () => {
		expect(moveAccount(ids, 'a', 'd', 'after')).toEqual(['b', 'c', 'd', 'a']);
		expect(moveAccount(ids, 'd', 'a', 'before')).toEqual(['d', 'a', 'b', 'c']);
	});

	it('changes nothing when the row is dropped beside itself', () => {
		expect(moveAccount(ids, 'b', 'b', 'before')).toBeNull();
		expect(moveAccount(ids, 'b', 'a', 'after')).toBeNull();
		expect(moveAccount(ids, 'b', 'c', 'before')).toBeNull();
	});

	it('ignores ids that are not in the list', () => {
		expect(moveAccount(ids, 'x', 'a', 'before')).toBeNull();
		expect(moveAccount(ids, 'a', 'x', 'after')).toBeNull();
	});
});
