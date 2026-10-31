import type { PooledWindow } from '$lib/accounts';

/** A pooled window as the core reports it; override only what a test cares about. */
export const pooledWindow = (over: Partial<PooledWindow> = {}): PooledWindow => ({
	remainingPercent: null,
	available: 0,
	total: 0,
	soonestReset: null,
	staleSinceMs: null,
	...over
});
