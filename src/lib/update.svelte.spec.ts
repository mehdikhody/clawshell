import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { check } from '@tauri-apps/plugin-updater';
import { UPDATE_CHECK_MS, checkForUpdate, startUpdateChecks, updateState } from './update.svelte';

vi.mock('@tauri-apps/plugin-updater', () => ({ check: vi.fn() }));

const checkMock = vi.mocked(check);
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
