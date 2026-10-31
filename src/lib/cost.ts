import { invoke } from '@tauri-apps/api/core';

/** Why there is no Code-tab estimate. */
export type NoEstimate = 'prices-unavailable';

/** One account's month. Estimate and total are null when there is no estimate: never a made-up zero. */
export type AccountCost = {
	id: string;
	label: string;
	/** Code-tab estimate, month to date, in dollars. */
	estimateUsd: number | null;
	/** Extra-usage spend, month to date, in dollars. */
	extraUsd: number;
	totalUsd: number | null;
	/** This account's part of the month total, 0 to 1. */
	share: number | null;
};

export type MonthCost = {
	estimateUsd: number | null;
	noEstimate: NoEstimate | null;
	extraUsd: number;
	/** Estimate plus extra usage; null when there is no estimate. */
	totalUsd: number | null;
	/** Messages left out of the estimate because their model has no price. */
	unpricedMessages: number;
	/** Biggest first. */
	accounts: AccountCost[];
};

/** This month's dollars across accounts; may fetch model prices, so it can take a moment. */
export const monthCost = () => invoke<MonthCost>('month_cost');

export const formatUsd = (amount: number, locale: string): string =>
	new Intl.NumberFormat(locale, { style: 'currency', currency: 'USD' }).format(amount);

/** A share (0 to 1) as a whole percent. */
export const formatShare = (share: number, locale: string): string =>
	new Intl.NumberFormat(locale, { style: 'percent', maximumFractionDigits: 0 }).format(share);
