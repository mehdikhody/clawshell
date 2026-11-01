// Injected by capture.mjs before the app loads: a stand-in Tauri backend answering with sample data.
// Config comes from window.__SHOT__ = { language, routingNeedsChoice, update }.
// `update` offers a new version; without it the header shows no "Update available".
// The accounts, usage, costs and sessions here are what the README screenshots show.
(() => {
	const cfg = window.__SHOT__ || {};
	const now = Date.now();
	const iso = (ms) => new Date(ms).toISOString();
	const H = 3600e3;
	const D = 24 * H;

	// ---- icons -----------------------------------------------------------
	const spark = (cx, cy, r) => {
		const rays = [1, 0.78, 0.95, 0.7, 1, 0.82, 0.92, 0.74, 0.98, 0.76, 0.9, 0.8];
		return rays
			.map((len, i) => {
				const a = (i / rays.length) * Math.PI * 2 - Math.PI / 2;
				const x = cx + Math.cos(a) * r * len;
				const y = cy + Math.sin(a) * r * len;
				return `<line x1="${cx}" y1="${cy}" x2="${x.toFixed(1)}" y2="${y.toFixed(1)}"/>`;
			})
			.join('');
	};
	const base = `<rect width="256" height="256" rx="56" fill="#d97757"/><g stroke="#fff" stroke-width="22" stroke-linecap="round">${spark(128, 128, 82)}</g>`;
	const svg = (inner) =>
		'data:image/svg+xml;base64,' +
		btoa(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">${inner}</svg>`);
	const claudeIcon = svg(base);
	const accountIcon = (badge, colour) => {
		const cx = 0.7 * 256,
			r = 0.25 * 256,
			ring = 0.045 * 256;
		// Shrink the spark so the disc doesn't swallow it, as the real icon does.
		const small = `<rect width="256" height="256" rx="56" fill="#d97757"/><g stroke="#fff" stroke-width="20" stroke-linecap="round">${spark(100, 100, 66)}</g>`;
		return svg(
			`${small}<circle cx="${cx}" cy="${cx}" r="${r + ring}" fill="#fff"/><circle cx="${cx}" cy="${cx}" r="${r}" fill="${colour}"/>` +
				`<text x="${cx}" y="${cx}" dy="0.35em" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-weight="700" font-size="${r * 1.15}" fill="#fff">${badge}</text>`
		);
	};

	// ---- accounts --------------------------------------------------------
	const snap = (five, fiveIn, week, weekIn, extra) => ({
		fiveHour: { utilization: five, resetsAt: iso(now + fiveIn) },
		weekly: { utilization: week, resetsAt: iso(now + weekIn) },
		extraUsage: extra ?? { enabled: false, usedMinor: null, limitMinor: null },
		fetchedAtMs: now - 40e3,
		source: 'live'
	});
	const account = (o) => ({
		isDefault: false,
		copyBlocker: null,
		running: o.status === 'running',
		usagePending: false,
		env: [],
		flags: [],
		...o,
		badge: o.label[0].toUpperCase()
	});
	const accounts = [
		account({
			id: 'default',
			label: 'Personal',
			isDefault: true,
			colour: '#2563eb',
			status: 'running',
			email: 'alex@example.com',
			usage: snap(64, 1.2 * H, 71, 2.3 * D + 3 * H, {
				enabled: true,
				usedMinor: 1250,
				limitMinor: 5000
			})
		}),
		account({
			id: 'b4c1',
			label: 'Work',
			colour: '#16a34a',
			status: 'running',
			email: 'alex@company.example',
			usage: snap(38, 2.6 * H, 46, 4.5 * D)
		}),
		account({
			id: 'c7d2',
			label: 'Side project',
			colour: '#9333ea',
			status: 'idle',
			email: 'builds@example.org',
			usage: snap(12, 3.4 * H, 23, 3.1 * D)
		}),
		account({
			id: 'e9f3',
			label: 'Research',
			colour: '#ea580c',
			status: 'limited',
			email: 'lab@example.net',
			copyBlocker: 'open-code-tab',
			usage: snap(100, 0.6 * H, 88, 1.4 * D)
		})
	];
	const pooledWindow = (key) => {
		const used = accounts.map((a) => a.usage[key]);
		return {
			remainingPercent: used.reduce((s, w) => s + (100 - w.utilization), 0) / used.length,
			available: used.filter((w) => w.utilization < 100).length,
			total: used.length,
			soonestReset: used.map((w) => w.resetsAt).sort()[0],
			staleSinceMs: null
		};
	};
	const accountList = {
		claudeFound: true,
		refreshAllowedAt: null,
		pooled: { fiveHour: pooledWindow('fiveHour'), weekly: pooledWindow('weekly') },
		palette: [
			'#2563eb',
			'#dc2626',
			'#16a34a',
			'#9333ea',
			'#ea580c',
			'#0891b2',
			'#db2777',
			'#a16207',
			'#4f46e5',
			'#0d9488'
		],
		accounts
	};

	// ---- cost ------------------------------------------------------------
	const shares = [0.46, 0.31, 0.15, 0.08];
	const monthCost = {
		estimateUsd: 390.74,
		noEstimate: null,
		extraUsd: 12.5,
		totalUsd: 403.24,
		unpricedMessages: 0,
		tokens: 327_800_000,
		accounts: accounts.map((a, i) => {
			const est = +(390.74 * shares[i]).toFixed(2);
			const extra = i === 0 ? 12.5 : 0;
			return {
				id: a.id,
				label: a.label,
				colour: a.colour,
				estimateUsd: est,
				extraUsd: extra,
				totalUsd: +(est + extra).toFixed(2),
				share: (est + extra) / 403.24,
				tokens: Math.round(327_800_000 * shares[i])
			};
		})
	};

	// ---- heatmap ---------------------------------------------------------
	let seed = 7;
	const rand = () => (seed = (seed * 16807) % 2147483647) / 2147483647;
	const today = new Date();
	today.setHours(0, 0, 0, 0);
	const first = new Date(today);
	first.setDate(first.getDate() - 56 - today.getDay());
	const days = [];
	for (let d = new Date(first); d <= today; d.setDate(d.getDate() + 1)) {
		const date = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
		const weekday = d.getDay();
		const weekend = weekday === 0 || weekday === 6;
		let usd = weekend ? 2 + rand() * 9 : 9 + rand() * 30;
		if (weekday === 5) usd += 12;
		if (rand() < 0.04) usd = 0;
		const split = (total, round) => {
			const w = accounts.map((_, i) => shares[i] * (0.6 + rand() * 0.8));
			const sum = w.reduce((s, x) => s + x, 0);
			return accounts
				.map((a, i) => ({
					id: a.id,
					label: a.label,
					colour: a.colour,
					value: round((total * w[i]) / sum)
				}))
				.filter((x) => x.value > 0)
				.sort((x, y) => y.value - x.value);
		};
		const tokens = Math.round(usd * 820_000);
		days.push({
			date,
			usd: { total: +usd.toFixed(2), accounts: split(usd, (v) => +v.toFixed(2)) },
			tokens: { total: tokens, accounts: split(tokens, Math.round) }
		});
	}
	const heatmap = { days, noEstimate: null };

	// ---- sessions --------------------------------------------------------
	const sessions = [
		['Fix flaky upload retry test', 'C:\\Users\\alex\\code\\uploader', 0.4 * H, false],
		['Add dark mode to settings page', 'C:\\Users\\alex\\code\\dashboard', 3 * H, false],
		['Migrate auth to OAuth PKCE', 'C:\\Users\\alex\\code\\api', 1.1 * D, false],
		['Write release notes for 2.4', 'C:\\Users\\alex\\code\\dashboard', 2.3 * D, false],
		['Profile slow CSV import', 'C:\\Users\\alex\\code\\importer', 4 * D, false],
		['Sketch landing page copy', 'C:\\Users\\alex\\AppData\\scratch-workspaces\\a1', 6 * D, false],
		['Old spike: GraphQL gateway', 'C:\\Users\\alex\\code\\gateway', 19 * D, true]
	].map(([title, cwd, ago, isArchived], i) => ({
		cliSessionId: `s${i}`,
		title,
		cwd,
		lastActivityAt: now - ago,
		isArchived
	}));

	// ---- settings --------------------------------------------------------
	let settings = {
		launchAtBoot: true,
		startHidden: false,
		closeToTray: true,
		language: cfg.language || 'en',
		calendar: 'automatic',
		claudePath: null,
		notifyWarning: true,
		notifyLimited: true,
		notifyReset: true,
		warningThreshold: 80,
		limitThreshold: 95,
		signInRouting: true,
		disclaimerAccepted: 1
	};

	const notes = [
		'Arabic, Russian and Simplified Chinese',
		'Calendar setting: Jalali, Hijri and Chinese lunar',
		'Copy Code sessions between accounts',
		'Quit Claude from an account’s menu',
		'Loading placeholders that keep their size',
		'Live drag and drop for accounts'
	].join('\n');

	const handlers = {
		list_accounts: () => accountList,
		refresh_usage: () => ({ refreshed: accounts.length, refreshAllowedAt: null }),
		account_icon: ({ badge, colour }) => accountIcon(badge, colour),
		claude_icon: () => claudeIcon,
		month_cost: () => monthCost,
		heatmap: () => heatmap,
		get_settings: () => settings,
		update_settings: ({ settings: next }) => (settings = next),
		routing_needs_choice: () => !!cfg.routingNeedsChoice,
		disclaimer_status: () => ({ accepted: true, version: 1 }),
		list_sessions: () => sessions,
		new_account_defaults: () => ({ label: 'Account 5', colour: '#0891b2' }),
		'plugin:updater|check': () =>
			cfg.update
				? {
						rid: 1,
						currentVersion: '0.2.0',
						version: '0.3.0',
						date: iso(now),
						body: notes,
						rawJson: {}
					}
				: null,
		'plugin:event|listen': () => Math.floor(Math.random() * 1e9),
		'plugin:event|unlisten': () => null,
		'plugin:resources|close': () => null
	};

	// ---- IPC plumbing (mirrors @tauri-apps/api/mocks) --------------------
	const callbacks = new Map();
	window.__TAURI_INTERNALS__ = {
		metadata: {
			currentWindow: { label: 'main' },
			currentWebview: { windowLabel: 'main', label: 'main' }
		},
		transformCallback(cb, once) {
			const id = Math.floor(Math.random() * 1e9);
			callbacks.set(id, (data) => {
				if (once) callbacks.delete(id);
				return cb && cb(data);
			});
			return id;
		},
		unregisterCallback(id) {
			callbacks.delete(id);
		},
		runCallback(id, data) {
			callbacks.get(id)?.(data);
		},
		callbacks,
		convertFileSrc: (p) => p,
		async invoke(cmd, args) {
			const h = handlers[cmd];
			if (!h) {
				console.warn('[mock] unhandled', cmd, args);
				return null;
			}
			return h(args || {});
		}
	};
	window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
})();
