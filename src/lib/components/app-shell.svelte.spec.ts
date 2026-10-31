import { page } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import '../../routes/layout.css';
import { mockTauri } from '$lib/test/tauri-ipc';
import AppShell from './app-shell.svelte';

let restore: () => void;

// The Accounts tab asks the core for its list as soon as it mounts.
const core = { list_accounts: () => ({ claudeFound: true, accounts: [] }) };
afterEach(() => restore?.());

describe('AppShell', () => {
	it('shows the Usage, Accounts and Settings tabs and switches screens', async () => {
		({ restore } = mockTauri(core));
		render(AppShell);

		const usage = page.getByRole('tab', { name: 'Usage' });
		const accounts = page.getByRole('tab', { name: 'Accounts' });
		const settings = page.getByRole('tab', { name: 'Settings' });

		await expect.element(usage).toHaveAttribute('aria-selected', 'true');
		await expect.element(page.getByRole('tabpanel', { name: 'Usage' })).toBeVisible();

		await accounts.click();
		await expect.element(accounts).toHaveAttribute('aria-selected', 'true');
		await expect.element(page.getByRole('tabpanel', { name: 'Accounts' })).toBeVisible();

		await settings.click();
		await expect.element(settings).toHaveAttribute('aria-selected', 'true');
		await expect.element(page.getByRole('tabpanel', { name: 'Settings' })).toBeVisible();
	});

	it('shows the app name and minimize and close buttons, but no maximize', async () => {
		({ restore } = mockTauri(core));
		render(AppShell);

		await expect.element(page.getByText('clawshell')).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Minimize' })).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Close' })).toBeVisible();
		await expect
			.element(page.getByRole('button', { name: /maximi[sz]e/i }))
			.not.toBeInTheDocument();
	});

	it('minimizes and closes the window through the top bar', async () => {
		const tauri = mockTauri(core);
		({ restore } = tauri);
		render(AppShell);

		await page.getByRole('button', { name: 'Minimize' }).click();
		await page.getByRole('button', { name: 'Close' }).click();

		const commands = tauri.calls.map((c) => c.cmd);
		expect(commands).toContain('plugin:window|minimize');
		expect(commands).toContain('plugin:window|close');
	});
});
