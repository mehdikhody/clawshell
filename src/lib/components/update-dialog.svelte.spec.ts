import { page, userEvent } from 'vitest/browser';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import type { Channel } from '@tauri-apps/api/core';
import { Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import '../../routes/layout.css';
import { switchLocale } from '$lib/locale-state.svelte';
import { mockTauri } from '$lib/test/tauri-ipc';
import { updateState } from '$lib/update.svelte';
import TopBar from './top-bar.svelte';

let restore: () => void;

afterEach(() => {
	restore?.();
	switchLocale('en');
	updateState.current = { status: 'idle' };
});

const notes = 'Fixed sign-in routing.\nUsage bars are faster.';
const release = () =>
	new Update({ rid: 7, currentVersion: '0.1.0', version: '0.2.0', body: notes, rawJson: {} });
const badge = () => page.getByRole('button', { name: 'Update available' });
const dialog = () => page.getByRole('dialog');

/** Sends one updater event down the channel the plugin handed to the download command. */
const send = (args: unknown, event: DownloadEvent) =>
	(args as { onEvent: Channel<DownloadEvent> }).onEvent.onmessage(event);

/** A download the test paces by hand: `step` runs the next move once the dialog has been checked. */
function pacedDownload() {
	let args: unknown;
	let finish: () => void = () => {};
	let fail: (reason: string) => void = () => {};
	const done = new Promise<void>((resolve, reject) => {
		finish = resolve;
		fail = reject;
	});
	return {
		handler: (a: unknown) => {
			args = a;
			return done;
		},
		send: (event: DownloadEvent) => send(args, event),
		finish: () => finish(),
		fail: (reason: string) => fail(reason)
	};
}

describe('update dialog', () => {
	it('opens from the badge with the new version and the release notes', async () => {
		({ restore } = mockTauri());
		updateState.current = { status: 'available', update: release() };
		render(TopBar);

		await expect.element(dialog()).not.toBeInTheDocument();
		await badge().click();

		await expect.element(dialog()).toBeVisible();
		await expect.element(page.getByRole('heading', { name: /0\.2\.0/ })).toBeVisible();
		await expect.element(dialog().getByText(/Fixed sign-in routing\./)).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Update', exact: true })).toBeVisible();
	});

	it('shows the release notes in a scrollable area', async () => {
		({ restore } = mockTauri());
		updateState.current = {
			status: 'available',
			update: new Update({
				rid: 7,
				currentVersion: '0.1.0',
				version: '0.2.0',
				body: Array.from({ length: 80 }, (_, i) => `Change ${i}`).join('\n'),
				rawJson: {}
			})
		};
		render(TopBar);
		await badge().click();

		const area = dialog().getByTestId('update-notes').element();
		expect(getComputedStyle(area).overflowY).toBe('auto');
		expect(area.scrollHeight).toBeGreaterThan(area.clientHeight);
	});

	it('closes with Later and keeps the badge', async () => {
		({ restore } = mockTauri());
		updateState.current = { status: 'available', update: release() };
		render(TopBar);
		await badge().click();

		await page.getByRole('button', { name: 'Later' }).click();

		await expect.element(dialog()).not.toBeInTheDocument();
		await expect.element(badge()).toBeVisible();
		expect(updateState.current.status).toBe('available');
	});

	it('shows download progress as a percentage, then installs and relaunches', async () => {
		const download = pacedDownload();
		const tauri = mockTauri({ 'plugin:updater|download_and_install': download.handler });
		({ restore } = tauri);
		updateState.current = { status: 'available', update: release() };
		render(TopBar);
		await badge().click();

		await page.getByRole('button', { name: 'Update', exact: true }).click();
		await vi.waitFor(() =>
			expect(tauri.calls.map((c) => c.cmd)).toContain('plugin:updater|download_and_install')
		);
		download.send({ event: 'Started', data: { contentLength: 200 } });
		download.send({ event: 'Progress', data: { chunkLength: 50 } });

		const bar = page.getByRole('progressbar');
		await expect.element(bar).toHaveAttribute('aria-valuenow', '25');
		await expect.element(dialog().getByText('Downloading update…')).toBeVisible();
		await expect.element(dialog().getByText('25%')).toBeVisible();

		download.send({ event: 'Progress', data: { chunkLength: 150 } });
		download.send({ event: 'Finished' });
		await expect.element(dialog().getByText('Installing and restarting…')).toBeVisible();

		download.finish();
		await vi.waitFor(() =>
			expect(tauri.calls.map((c) => c.cmd)).toContain('plugin:process|restart')
		);
	});

	it('is indeterminate while the download size is unknown', async () => {
		const download = pacedDownload();
		const tauri = mockTauri({ 'plugin:updater|download_and_install': download.handler });
		({ restore } = tauri);
		updateState.current = { status: 'available', update: release() };
		render(TopBar);
		await badge().click();

		await page.getByRole('button', { name: 'Update', exact: true }).click();
		await vi.waitFor(() =>
			expect(tauri.calls.map((c) => c.cmd)).toContain('plugin:updater|download_and_install')
		);
		download.send({ event: 'Started', data: {} });
		download.send({ event: 'Progress', data: { chunkLength: 50 } });

		const bar = page.getByRole('progressbar');
		await expect.element(bar).toHaveAttribute('data-indeterminate');
		await expect.element(bar).not.toHaveAttribute('aria-valuenow');
		await expect.element(dialog().getByText('%')).not.toBeInTheDocument();
		download.finish();
	});

	it('keeps the dialog open while the update downloads', async () => {
		const download = pacedDownload();
		const tauri = mockTauri({ 'plugin:updater|download_and_install': download.handler });
		({ restore } = tauri);
		updateState.current = { status: 'available', update: release() };
		render(TopBar);
		await badge().click();
		await page.getByRole('button', { name: 'Update', exact: true }).click();
		await expect.element(page.getByRole('progressbar')).toBeVisible();

		await userEvent.keyboard('{Escape}');
		await userEvent.click(document.body, { position: { x: 2, y: 2 } });
		await expect.element(dialog()).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Later' })).not.toBeInTheDocument();
		await expect
			.element(page.getByRole('button', { name: 'Update', exact: true }))
			.not.toBeInTheDocument();
		download.finish();
	});

	it('closes with Escape when nothing is downloading', async () => {
		({ restore } = mockTauri());
		updateState.current = { status: 'available', update: release() };
		render(TopBar);
		await badge().click();

		await userEvent.keyboard('{Escape}');

		await expect.element(dialog()).not.toBeInTheDocument();
	});

	it('does not reopen by itself when an update appears after the state went idle', async () => {
		({ restore } = mockTauri());
		updateState.current = { status: 'available', update: release() };
		render(TopBar);
		await badge().click();
		await expect.element(dialog()).toBeVisible();

		updateState.current = { status: 'idle' };
		await expect.element(dialog()).not.toBeInTheDocument();
		updateState.current = { status: 'available', update: release() };

		await expect.element(badge()).toBeVisible();
		await expect.element(dialog()).not.toBeInTheDocument();
	});

	it('shows the error and Retry when the download fails, and Retry downloads again', async () => {
		let attempt = 0;
		const tauri = mockTauri({
			'plugin:updater|download_and_install': () => {
				attempt += 1;
				if (attempt === 1) throw 'network down';
			}
		});
		({ restore } = tauri);
		updateState.current = { status: 'available', update: release() };
		render(TopBar);
		await badge().click();

		await page.getByRole('button', { name: 'Update', exact: true }).click();

		await expect.element(dialog().getByText('Update failed')).toBeVisible();
		await expect.element(dialog().getByText(/current version is still running/)).toBeVisible();
		await expect.element(dialog().getByText('network down')).not.toBeInTheDocument();
		expect(tauri.calls.map((c) => c.cmd)).not.toContain('plugin:process|restart');

		await page.getByRole('button', { name: 'Retry' }).click();

		await vi.waitFor(() =>
			expect(tauri.calls.map((c) => c.cmd)).toContain('plugin:process|restart')
		);
		expect(attempt).toBe(2);
	});

	it('can be closed with Later after a failure, leaving the badge', async () => {
		({ restore } = mockTauri({
			'plugin:updater|download_and_install': () => {
				throw 'network down';
			}
		}));
		updateState.current = { status: 'available', update: release() };
		render(TopBar);
		await badge().click();
		await page.getByRole('button', { name: 'Update', exact: true }).click();
		await expect.element(page.getByRole('button', { name: 'Retry' })).toBeVisible();

		await page.getByRole('button', { name: 'Later' }).click();

		await expect.element(dialog()).not.toBeInTheDocument();
		await expect.element(badge()).toBeVisible();
	});

	it('describes a failure in Persian, without the raw error', async () => {
		({ restore } = mockTauri({
			'plugin:updater|download_and_install': () => {
				throw 'network down';
			}
		}));
		switchLocale('fa');
		updateState.current = { status: 'available', update: release() };
		render(TopBar);
		await page.getByRole('button', { name: 'به‌روزرسانی در دسترس است' }).click();

		await page.getByRole('button', { name: 'به‌روزرسانی', exact: true }).click();

		await expect.element(dialog().getByText('به‌روزرسانی انجام نشد')).toBeVisible();
		await expect.element(dialog().getByText(/نسخه فعلی/)).toBeVisible();
		await expect.element(dialog().getByText('network down')).not.toBeInTheDocument();
	});

	it('renders in Persian, right to left', async () => {
		({ restore } = mockTauri());
		switchLocale('fa');
		updateState.current = { status: 'available', update: release() };
		render(TopBar);

		await page.getByRole('button', { name: 'به‌روزرسانی در دسترس است' }).click();

		await expect.element(dialog()).toBeVisible();
		await expect.element(page.getByRole('heading', { name: /۰\.۲\.۰|0\.2\.0/ })).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'بعداً' })).toBeVisible();
		await expect
			.element(page.getByRole('button', { name: 'به‌روزرسانی', exact: true }))
			.toBeVisible();
		expect(getComputedStyle(dialog().element()).direction).toBe('rtl');
	});
});
