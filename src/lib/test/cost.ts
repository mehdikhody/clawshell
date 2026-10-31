import type { AccountCost, MonthCost } from '$lib/cost';

/** A month as the core reports it; override only what a test cares about. */
export const monthCost = (over: Partial<MonthCost> = {}): MonthCost => ({
	estimateUsd: 0,
	noEstimate: null,
	extraUsd: 0,
	totalUsd: 0,
	unpricedMessages: 0,
	tokens: 0,
	accounts: [],
	...over
});

export const accountCost = (id: string, over: Partial<AccountCost> = {}): AccountCost => ({
	id,
	label: id,
	estimateUsd: 0,
	extraUsd: 0,
	totalUsd: 0,
	share: null,
	tokens: 0,
	...over
});
