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

/** One environment variable of an account's Claude process. The value can be a secret. */
export type EnvVar = { name: string; value: string };

/** What an account adds to every start of its Claude. */
export type LaunchOptions = {
	env: EnvVar[];
	/** Extra command-line arguments, one argument per entry. */
	flags: string[];
};

export type Account = {
	/** The uuid of a clawshell profile, or `default`. */
	id: string;
	label: string;
	/**
	 * Claude's own profile, the one its Start-menu entry opens. Always first; it has no badge,
	 * shortcuts or drag handle, and can't be deleted.
	 */
	isDefault: boolean;
	/** The first letter of the label, in capitals. */
	badge: string;
	/** The badge colour, `#rrggbb`. */
	colour: string;
	status: AccountStatus;
	/** A Claude Desktop window is open on this account, whatever the badge says. */
	running: boolean;
	email: string | null;
	usage: UsageSnapshot | null;
	env: EnvVar[];
	flags: string[];
};

/** One window pooled over the signed-in accounts that report it. */
export type PooledWindow = {
	/** Average of (100 - utilisation); null when no account reports the window. */
	remainingPercent: number | null;
	/** Accounts below 100% used: N in "N of M accounts available". */
	available: number;
	/** Accounts counted: M. */
	total: number;
	/** Earliest reset among them (RFC 3339); null when none has a reset time. */
	soonestReset: string | null;
	/** Epoch ms the oldest not-live input was taken; null when every input is live. */
	staleSinceMs: number | null;
};

export type PooledRemaining = {
	fiveHour: PooledWindow;
	weekly: PooledWindow;
};

/** Nothing pooled yet: no signed-in account has a reading. */
export const emptyPooled: PooledRemaining = {
	fiveHour: {
		remainingPercent: null,
		available: 0,
		total: 0,
		soonestReset: null,
		staleSinceMs: null
	},
	weekly: { remainingPercent: null, available: 0, total: 0, soonestReset: null, staleSinceMs: null }
};

export type AccountList = {
	/** False when Claude Desktop can't be found. */
	claudeFound: boolean;
	/** Epoch ms before which a manual usage refresh is refused; null when allowed now. */
	refreshAllowedAt: number | null;
	/** The 5-hour and weekly windows pooled over the signed-in accounts. */
	pooled: PooledRemaining;
	/** The badge colours the picker offers. */
	palette: string[];
	accounts: Account[];
};

export type RefreshOutcome = {
	refreshed: number;
	refreshAllowedAt: number | null;
};

/** What a failed command rejects with; `code` is stable and localisable. */
export type CommandError = { code: string; message: string };

export const listAccounts = () => invoke<AccountList>('list_accounts');

const accountIcons = new Map<string, Promise<string>>();

/** The account's badge icon as an image URL, fetched once per badge and colour. */
export function accountIcon(badge: string, colour: string): Promise<string> {
	const key = `${badge}|${colour}`;
	let icon = accountIcons.get(key);
	if (!icon) {
		icon = invoke<string>('account_icon', { badge, colour });
		accountIcons.set(key, icon);
		// A failed fetch is retried the next time the row is drawn.
		icon.catch(() => accountIcons.delete(key));
	}
	return icon;
}

let plainIcon: Promise<string> | null = null;

/** Claude's own icon, without a badge, as an image URL: the default account's. Fetched once. */
export function claudeIcon(): Promise<string> {
	if (!plainIcon) {
		const icon = invoke<string>('claude_icon');
		plainIcon = icon;
		// A failed fetch is retried the next time the row is drawn.
		icon.catch(() => (plainIcon = null));
	}
	return plainIcon;
}

/** The name and badge colour the Add dialog starts with. */
export type NewAccount = { label: string; colour: string };

export const newAccountDefaults = () => invoke<NewAccount>('new_account_defaults');
/** Creates an account under `label` and `colour`. Claude is not started on it. */
export const addAccount = (label: string, colour: string) =>
	invoke<Account>('add_account', { label, colour });
export const launchAccount = (id: string) =>
	invoke<'started' | 'focused'>('launch_account', { id });
export const renameAccount = (id: string, label: string) =>
	invoke<Account>('rename_account', { id, label });
/** Saves the account order, top to bottom, as `ids` lists the accounts. */
export const reorderAccounts = (ids: string[]) => invoke<void>('reorder_accounts', { ids });
export const setAccountColour = (id: string, colour: string) =>
	invoke<Account>('set_account_colour', { id, colour });
/** Saves the account's env vars and flags, or rejects with a code such as `invalid-env-name`. */
export const setLaunchOptions = (id: string, options: LaunchOptions) =>
	invoke<Account>('set_launch_options', { id, options });
export const openAccountFolder = (id: string) => invoke<void>('open_account_folder', { id });
export const deleteAccount = (id: string) => invoke<void>('delete_account', { id });
/** Puts "Claude - <name>" shortcuts on the Desktop and in the Start menu. */
export const addShortcuts = (id: string) => invoke<void>('add_shortcuts', { id });
/** Manual usage refresh; the core refuses it inside the minimum interval. */
export const refreshUsage = () => invoke<RefreshOutcome>('refresh_usage');

/** Calls `handler` whenever the core reports a changed account list. */
export const onAccountsChanged = (handler: (list: AccountList) => void): Promise<UnlistenFn> =>
	listen<AccountList>('accounts-changed', (event) => handler(event.payload));

/** Calls `handler` when the tray asks for a new account. */
export const onAddAccountRequested = (handler: () => void): Promise<UnlistenFn> =>
	listen('add-account-requested', () => handler());

export function errorCode(error: unknown): string | undefined {
	return typeof error === 'object' && error !== null && 'code' in error
		? String((error as CommandError).code)
		: undefined;
}
