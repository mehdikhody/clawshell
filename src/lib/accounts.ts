import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** Mirrors the core's `Status`. `missing`: the profile folder was deleted outside clawshell. */
export type AccountStatus = 'running' | 'idle' | 'signed-out' | 'missing';

export type Account = {
	id: string;
	label: string;
	status: AccountStatus;
	/** A Claude Desktop window is open on this account, whatever the badge says. */
	running: boolean;
};

export type AccountList = {
	/** False when Claude Desktop can't be found. */
	claudeFound: boolean;
	accounts: Account[];
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

/** Calls `handler` whenever the core reports a changed account list. */
export const onAccountsChanged = (handler: (list: AccountList) => void): Promise<UnlistenFn> =>
	listen<AccountList>('accounts-changed', (event) => handler(event.payload));

export function errorCode(error: unknown): string | undefined {
	return typeof error === 'object' && error !== null && 'code' in error
		? String((error as CommandError).code)
		: undefined;
}
