import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { check, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import {
	UPDATE_CHECK_MS,
	checkForUpdate,
	installUpdate,
	startUpdateChecks,
	updateState
} from './update.svelte';

vi.mock('@tauri-apps/plugin-updater', () => ({ check: vi.fn() }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));

const checkMock = vi.mocked(check);
const relaunchMock = vi.mocked(relaunch);
const found = (version: string) =>
	({
		version,
		currentVersion: '0.1.0',
		body: 'Notes',
		close: vi.fn().mockResolvedValue(undefined)
	}) as never;

beforeEach(() => {
	vi.useFakeTimers();
	checkMock.mockReset();
	relaunchMock.mockReset();
	updateState.current = { status: 'idle' };
});
afterEach(() => vi.useRealTimers());

describe('checkForUpdate', () => {
	it('moves to available when a newer release exists', async () => {
		const update = found('0.2.0');
		checkMock.mockResolvedValue(update);

		await checkForUpdate();

		expect(updateState.current).toEqual({ status: 'available', update });
	});

	it('stays idle when nothing newer exists', async () => {
		checkMock.mockResolvedValue(null);

		await checkForUpdate();

		expect(updateState.current.status).toBe('idle');
	});

	it('swallows a failed check and leaves the state as it was', async () => {
		const update = found('0.2.0');
		updateState.current = { status: 'available', update };
		checkMock.mockRejectedValue(new Error('offline'));

		await expect(checkForUpdate()).resolves.toBeUndefined();

		expect(updateState.current).toEqual({ status: 'available', update });
	});

	it('clears an earlier badge when nothing newer exists', async () => {
		updateState.current = { status: 'available', update: found('0.2.0') };
		checkMock.mockResolvedValue(null);

		await checkForUpdate();

		expect(updateState.current.status).toBe('idle');
	});

	it('keeps a download in progress when nothing newer exists', async () => {
		const state = {
			status: 'downloading' as const,
			update: found('0.2.0'),
			downloaded: 10,
			total: 100
		};
		updateState.current = state;
		checkMock.mockResolvedValue(null);

		await checkForUpdate();

		expect(updateState.current).toEqual(state);
	});

	it('keeps a download in progress when a check finds another release', async () => {
		const state = {
			status: 'downloading' as const,
			update: found('0.2.0'),
			downloaded: 10,
			total: 100
		};
		updateState.current = state;
		checkMock.mockResolvedValue(found('0.3.0'));

		await checkForUpdate();

		expect(updateState.current).toEqual(state);
	});

	it('keeps an install in progress when nothing newer exists', async () => {
		const state = { status: 'installing' as const, update: found('0.2.0') };
		updateState.current = state;
		checkMock.mockResolvedValue(null);

		await checkForUpdate();

		expect(updateState.current).toEqual(state);
	});
});

describe('startUpdateChecks', () => {
	it('checks once at startup, then every 6 hours', async () => {
		checkMock.mockResolvedValue(null);

		const stop = startUpdateChecks();
		expect(checkMock).toHaveBeenCalledTimes(1);

		await vi.advanceTimersByTimeAsync(UPDATE_CHECK_MS - 1);
		expect(checkMock).toHaveBeenCalledTimes(1);

		await vi.advanceTimersByTimeAsync(1);
		expect(checkMock).toHaveBeenCalledTimes(2);

		await vi.advanceTimersByTimeAsync(UPDATE_CHECK_MS);
		expect(checkMock).toHaveBeenCalledTimes(3);
		stop();
	});

	it('is 6 hours', () => {
		expect(UPDATE_CHECK_MS).toBe(21_600_000);
	});

	it('stops checking once stopped', async () => {
		checkMock.mockResolvedValue(null);

		const stop = startUpdateChecks();
		stop();
		await vi.advanceTimersByTimeAsync(UPDATE_CHECK_MS * 2);

		expect(checkMock).toHaveBeenCalledTimes(1);
	});

	it('keeps checking after a failed check', async () => {
		checkMock.mockRejectedValueOnce(new Error('offline')).mockResolvedValue(found('0.2.0'));

		const stop = startUpdateChecks();
		await vi.advanceTimersByTimeAsync(UPDATE_CHECK_MS);

		expect(updateState.current.status).toBe('available');
		stop();
	});
});

describe('installUpdate', () => {
	type Run = (onEvent: (event: DownloadEvent) => void) => Promise<void>;
	const updateWith = (run: Run) =>
		({
			version: '0.2.0',
			currentVersion: '0.1.0',
			body: 'Notes',
			close: vi.fn().mockResolvedValue(undefined),
			downloadAndInstall: vi.fn(run)
		}) as never;
	const snapshot = () => ({ ...updateState.current });
	const installCalls = (update: unknown) =>
		vi.mocked((update as { downloadAndInstall: () => void }).downloadAndInstall).mock.calls.length;

	it('walks through the download progress to the install, then relaunches', async () => {
		const seen: unknown[] = [];
		const update = updateWith(async (onEvent) => {
			onEvent({ event: 'Started', data: { contentLength: 200 } });
			seen.push(snapshot());
			onEvent({ event: 'Progress', data: { chunkLength: 50 } });
			onEvent({ event: 'Progress', data: { chunkLength: 70 } });
			seen.push(snapshot());
			onEvent({ event: 'Finished' });
			seen.push(snapshot());
		});
		updateState.current = { status: 'available', update };
		relaunchMock.mockImplementation(async () => {
			seen.push(snapshot());
		});

		await installUpdate();

		expect(seen).toEqual([
			{ status: 'downloading', update, downloaded: 0, total: 200 },
			{ status: 'downloading', update, downloaded: 120, total: 200 },
			{ status: 'installing', update },
			{ status: 'installing', update }
		]);
		expect(relaunchMock).toHaveBeenCalledTimes(1);
	});

	it('keeps the total unknown when the server sends no content length', async () => {
		let seen: unknown;
		const update = updateWith(async (onEvent) => {
			onEvent({ event: 'Started', data: {} });
			onEvent({ event: 'Progress', data: { chunkLength: 30 } });
			seen = snapshot();
		});
		updateState.current = { status: 'available', update };

		await installUpdate();

		expect(seen).toEqual({ status: 'downloading', update, downloaded: 30, total: null });
	});

	it('is downloading before any event arrives', async () => {
		let seen: unknown;
		const update = updateWith(async () => {
			seen = snapshot();
		});
		updateState.current = { status: 'available', update };

		await installUpdate();

		expect(seen).toEqual({ status: 'downloading', update, downloaded: 0, total: null });
	});

	it('fails with the error message when the download fails, and does not relaunch', async () => {
		const update = updateWith(async () => {
			throw new Error('network down');
		});
		updateState.current = { status: 'available', update };

		await installUpdate();

		expect(updateState.current).toEqual({ status: 'failed', update, message: 'network down' });
		expect(relaunchMock).not.toHaveBeenCalled();
	});

	it('fails with a plain string error from the updater plugin', async () => {
		const update = updateWith(async () => {
			throw 'signature mismatch';
		});
		updateState.current = { status: 'available', update };

		await installUpdate();

		expect(updateState.current).toEqual({
			status: 'failed',
			update,
			message: 'signature mismatch'
		});
	});

	it('fails when the restart fails', async () => {
		const update = updateWith(async () => {});
		updateState.current = { status: 'available', update };
		relaunchMock.mockRejectedValue(new Error('no restart'));

		await installUpdate();

		expect(updateState.current).toEqual({ status: 'failed', update, message: 'no restart' });
	});

	it('downloads again from the failed state', async () => {
		const update = updateWith(async () => {});
		updateState.current = { status: 'failed', update, message: 'network down' };

		await installUpdate();

		expect(installCalls(update)).toBe(1);
		expect(relaunchMock).toHaveBeenCalledTimes(1);
	});

	it('ignores a second call while a download is running', async () => {
		const update = updateWith(async () => {});
		updateState.current = { status: 'downloading', update, downloaded: 5, total: 100 };

		await installUpdate();

		expect(installCalls(update)).toBe(0);
		expect(updateState.current.status).toBe('downloading');
	});

	it('does nothing when there is no update', async () => {
		await installUpdate();

		expect(updateState.current.status).toBe('idle');
		expect(relaunchMock).not.toHaveBeenCalled();
	});
});
