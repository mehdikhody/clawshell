import { describe, expect, it } from 'vitest';
import { formatShare, formatUsd } from './cost';

describe('formatUsd', () => {
	it('writes dollars with cents', () => {
		expect(formatUsd(14.5, 'en-US')).toBe('$14.50');
		expect(formatUsd(0, 'en-US')).toBe('$0.00');
	});

	it('groups thousands', () => {
		expect(formatUsd(1234.567, 'en-US')).toBe('$1,234.57');
	});
});

describe('formatShare', () => {
	it('rounds a share to a whole percent', () => {
		expect(formatShare(0.8182, 'en-US')).toBe('82%');
		expect(formatShare(1, 'en-US')).toBe('100%');
	});
});
