import { describe, expect, it } from 'vitest';
import { defaultThresholds, usageStage } from './usage-stage';

describe('usageStage', () => {
	it('is normal below the warning threshold', () => {
		expect(usageStage(0, defaultThresholds)).toBe('normal');
		expect(usageStage(79.9, defaultThresholds)).toBe('normal');
	});

	it('warms at the warning threshold and up to the limit', () => {
		expect(usageStage(80, defaultThresholds)).toBe('warning');
		expect(usageStage(99.9, defaultThresholds)).toBe('warning');
	});

	it('is limit at the limit threshold and above', () => {
		expect(usageStage(100, defaultThresholds)).toBe('limit');
		expect(usageStage(120, defaultThresholds)).toBe('limit');
	});

	it('follows the saved thresholds', () => {
		const custom = { warningThreshold: 50, limitThreshold: 90 };
		expect(usageStage(49, custom)).toBe('normal');
		expect(usageStage(50, custom)).toBe('warning');
		expect(usageStage(90, custom)).toBe('limit');
	});
});
