import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/**
 * Mirrors the core's `Status`. `limited`: a 5-hour or weekly window is at or above 100%.
 * `missing`: the profile folder was deleted outside clawshell.
 */
export type AccountStatus = 'running' | 'idle' | 'signed-out' | 'limited' | 'missing';

/** `stale`: the last live numbers, kept after a failed poll. `local-history`: from Claude's own file. */
export type UsageSource = 'live' | 'stale' | 'local-history';

export type WindowUsage = {
	/** Percent of the window used, 0 to 100. */
	utilization: number;
	/** RFC 3339 reset time; null when the source has none (local history). */
	resetsAt: string | null;
};

export type UsageSnapshot = {
	fiveHour: WindowUsage | null;
	weekly: WindowUsage | null;
	/** Extra-usage spend in minor currency units, when the account has it enabled. */
	extraUsage: { enabled: boolean; usedMinor: number | null; limitMinor: number | null } | null;
	/** Epoch ms the numbers were taken. */
	fetchedAtMs: number;
	source: UsageSource;
};

export type Account = {
	id: string;
	label: string;
	status: AccountStatus;
	/** A Claude Desktop window is open on this account, whatever the badge says. */
	running: boolean;
	email: string | null;
	usage: UsageSnapshot | null;
};

export type AccountList = {
	/** False when Claude Desktop can't be found. */
	claudeFound: boolean;
	/** Epoch ms before which a manual usage refresh is refused; null when allowed now. */
	refreshAllowedAt: number | null;
	accounts: Account[];
};

export type RefreshOutcome = {
	refreshed: number;
	refreshAllowedAt: number | null;
};

/** What a failed command rejects with; `code` is stable and localisable. */
export type CommandError = { code: string; message: string };

export const listAccounts = () => invoke<AccountList>('list_accounts');
export const addAccount = () => invoke<Account>('add_account');
export const launchAccount = (id: string) =>
	invoke<'started' | 'focused'>('launch_account', { id });
export const renameAccount = (id: string, label: string) =>
	invoke<Account>('rename_account', { id, label });
export const openAccountFolder = (id: string) => invoke<void>('open_account_folder', { id });
export const deleteAccount = (id: string) => invoke<void>('delete_account', { id });
/** Manual usage refresh; the core refuses it inside the minimum interval. */
export const refreshUsage = () => invoke<RefreshOutcome>('refresh_usage');

/** Calls `handler` whenever the core reports a changed account list. */
export const onAccountsChanged = (handler: (list: AccountList) => void): Promise<UnlistenFn> =>
	listen<AccountList>('accounts-changed', (event) => handler(event.payload));

export function errorCode(error: unknown): string | undefined {
	return typeof error === 'object' && error !== null && 'code' in error
		? String((error as CommandError).code)
		: undefined;
}
