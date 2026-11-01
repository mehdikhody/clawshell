// Retakes the README screenshots in docs/screenshots from the frontend, with sample data.
//
//   npm run screenshots              every screenshot
//   npm run screenshots -- usage     only the named ones (see `shots` below)
//
// The frontend runs in headless Chromium on the Vite dev server (started here unless one is
// already on port 1420), and mock.js stands in for the Tauri backend.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = join(import.meta.dirname, '..', '..');
const out = join(root, 'docs', 'screenshots');
const mock = readFileSync(join(import.meta.dirname, 'mock.js'), 'utf8');
const url = 'http://127.0.0.1:1420/';

async function devServer() {
	try {
		await fetch(url);
		return null;
	} catch {
		const server = await createServer({ root, logLevel: 'warn' });
		await server.listen();
		return server;
	}
}

/** A fresh window the size of clawshell's (400×780) at 2x, settled on the first screen. */
async function open(browser, cfg = {}) {
	const context = await browser.newContext({
		viewport: { width: 400, height: 780 },
		deviceScaleFactor: 2,
		colorScheme: 'dark',
		reducedMotion: 'reduce'
	});
	await context.addInitScript(`window.__SHOT__ = ${JSON.stringify(cfg)};\n${mock}`);
	const page = await context.newPage();
	page.on('console', (message) => {
		if (message.type() === 'warning' || message.type() === 'error') {
			console.log(`  [${cfg.language ?? 'en'}] ${message.text()}`);
		}
	});
	await page.goto(url);
	await page.getByRole('tab').first().waitFor();
	await settle(page, 800);
	return page;
}

const settle = (page, ms = 700) => page.waitForTimeout(ms);
// By position, so the same code works in every language: Usage, Accounts, Settings.
const tab = (page, index) => page.getByRole('tab').nth(index).click();
const scrollTo = (locator, block = 'start') =>
	locator.evaluate((el, block) => el.scrollIntoView({ block }), block);

/** Each shot: the file it writes, the window config, and how to get there. */
const shots = {
	usage: {
		go: (page) => tab(page, 0)
	},
	accounts: {
		go: (page) => tab(page, 1)
	},
	settings: {
		go: (page) => tab(page, 2)
	},
	'copy-sessions': {
		async go(page) {
			await tab(page, 1);
			await settle(page);
			await page.getByText('alex@example.com').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Copy sessions…' }).click();
			await settle(page, 600);
			await page.locator('#copy-target').click();
			await page.getByRole('menuitemradio', { name: /Work/ }).click();
			await settle(page, 300);
			await page.locator('#copy-sessions').click();
			await settle(page, 300);
			for (const title of [
				'Fix flaky upload retry test',
				'Add dark mode to settings page',
				'Migrate auth to OAuth PKCE'
			]) {
				await page.getByRole('option', { name: new RegExp(title) }).click();
			}
			await page.mouse.move(0, 0);
		}
	},
	'account-menu': {
		async go(page) {
			await tab(page, 1);
			await settle(page);
			await page.getByText('alex@company.example').click({ button: 'right' });
		}
	},
	calendar: {
		async go(page) {
			await tab(page, 2);
			await settle(page);
			const button = page.locator('[aria-labelledby="setting-calendar"]');
			await scrollTo(button, 'center');
			await settle(page, 300);
			await button.click();
		}
	},
	persian: {
		cfg: { language: 'fa' },
		go: (page) => tab(page, 0)
	},
	arabic: {
		cfg: { language: 'ar' },
		go: (page) => tab(page, 1)
	},
	russian: {
		cfg: { language: 'ru' },
		go: (page) => tab(page, 0)
	},
	chinese: {
		cfg: { language: 'zh' },
		go: (page) => tab(page, 1)
	},
	update: {
		cfg: { update: true },
		async go(page) {
			await tab(page, 2);
			await settle(page);
			await page.getByRole('button', { name: 'Update available' }).click();
		}
	},
	'heatmap-tooltip': {
		async go(page) {
			await tab(page, 0);
			await settle(page);
			const cells = page.getByRole('group', { name: 'Last 9 weeks' }).getByRole('button');
			// A Friday two weeks back: a busy day with every account in it.
			await cells.nth(Math.min((await cells.count()) - 1, 7 * 6 + 5)).hover();
		}
	},
	notifications: {
		async go(page) {
			await tab(page, 2);
			await settle(page);
			await scrollTo(page.getByText('Notifications', { exact: true }));
		}
	},
	'sign-in-warning': {
		cfg: { routingNeedsChoice: true },
		async go(page) {
			await tab(page, 2);
			await settle(page);
			await scrollTo(page.getByRole('button', { name: 'Open Default Apps' }), 'center');
		}
	}
};

const only = process.argv.slice(2);
const unknown = only.filter((name) => !(name in shots));
if (unknown.length) {
	console.error(
		`Unknown screenshot: ${unknown.join(', ')}. Known: ${Object.keys(shots).join(', ')}`
	);
	process.exit(1);
}

const server = await devServer();
const browser = await chromium.launch();
try {
	for (const [name, shot] of Object.entries(shots)) {
		if (only.length && !only.includes(name)) continue;
		console.log(name);
		const page = await open(browser, shot.cfg);
		await shot.go(page);
		await settle(page);
		await page.screenshot({ path: join(out, `${name}.png`) });
		await page.context().close();
	}
} finally {
	await browser.close();
	await server?.close();
}
