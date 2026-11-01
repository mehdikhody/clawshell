import { invoke } from '@tauri-apps/api/core';

/** One Code session of an account. Mirrors the core's `CodeSession`. */
export type CodeSession = {
	cliSessionId: string;
	title: string;
	/** The folder the session works in; empty when the record names none. */
	cwd: string;
	/** Epoch ms of the last activity. */
	lastActivityAt: number;
	isArchived: boolean;
};

/** The account's Code sessions from every organisation, newest activity first. */
export const listSessions = (id: string) => invoke<CodeSession[]>('list_sessions', { id });

/** What a copy did. Mirrors the core's `CopyOutcome`. */
export type CopyOutcome = {
	copied: number;
	/** The target's Claude is running, so it lists the copies only after a restart. */
	targetRunning: boolean;
};

/** Copies the source's ticked sessions into the target account. */
export const copySessions = (sourceId: string, cliSessionIds: string[], targetId: string) =>
	invoke<CopyOutcome>('copy_sessions', { sourceId, cliSessionIds, targetId });

/** The last part of a session's folder path, or null for a session with no folder of its own. */
export function folderName(cwd: string): string | null {
	// Claude runs a session started without a folder in a scratch workspace.
	if (/[\\/]scratch-workspaces[\\/]/.test(cwd)) return null;
	return cwd.split(/[\\/]/).findLast((part) => part !== '') ?? null;
}
