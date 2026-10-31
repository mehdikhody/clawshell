import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';

/** How often the latest release is checked while the app runs. */
export const UPDATE_CHECK_MS = 6 * 60 * 60 * 1000;

/**
 * Where the update flow stands. The header badge shows for every status except `idle`.
 * `total` is null while the download size is unknown.
 */
export type UpdateStatus =
	| { status: 'idle' }
	| { status: 'available'; update: Update }
	| { status: 'downloading'; update: Update; downloaded: number; total: number | null }
	| { status: 'installing'; update: Update }
	| { status: 'failed'; update: Update; message: string };

/** The shared update state. Assign `current` to move the flow forward. */
export const updateState = $state<{ current: UpdateStatus }>({ current: { status: 'idle' } });

/** True while the header should offer the update: it is available or further along. */
export function hasUpdate(): boolean {
	return updateState.current.status !== 'idle';
}

/** Checks the latest release once. A failed or offline check changes nothing. */
export async function checkForUpdate(): Promise<void> {
	let found: Update | null;
	try {
		found = await check();
	} catch {
		return;
	}
	// A running download or install owns the state; a check never interrupts it.
	const { status } = updateState.current;
	if (status === 'downloading' || status === 'installing') {
		await found?.close().catch(() => {});
		return;
	}
	updateState.current = found ? { status: 'available', update: found } : { status: 'idle' };
}

/** Checks now, then every 6 hours. Returns a function that stops the checks. */
export function startUpdateChecks(): () => void {
	void checkForUpdate();
	const timer = setInterval(() => void checkForUpdate(), UPDATE_CHECK_MS);
	return () => clearInterval(timer);
}

function messageOf(error: unknown): string {
	if (error instanceof Error) return error.message;
	return typeof error === 'string' ? error : String(error);
}

/**
 * Downloads and installs the available update, then restarts clawshell.
 * Works from `available`, or from `failed` as a retry; any other state is left alone.
 * A failure at any step lands in `failed` and the current version keeps running.
 */
export async function installUpdate(): Promise<void> {
	const { current } = updateState;
	if (current.status !== 'available' && current.status !== 'failed') return;
	const { update } = current;

	let downloaded = 0;
	let total: number | null = null;
	updateState.current = { status: 'downloading', update, downloaded, total };
	try {
		await update.downloadAndInstall((event) => {
			if (event.event === 'Started') {
				total = event.data.contentLength ?? null;
				updateState.current = { status: 'downloading', update, downloaded, total };
			} else if (event.event === 'Progress') {
				downloaded += event.data.chunkLength;
				updateState.current = { status: 'downloading', update, downloaded, total };
			} else {
				updateState.current = { status: 'installing', update };
			}
		});
		updateState.current = { status: 'installing', update };
		await relaunch();
	} catch (error) {
		updateState.current = { status: 'failed', update, message: messageOf(error) };
	}
}
