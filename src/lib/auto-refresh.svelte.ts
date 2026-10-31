import { refreshUsage } from '$lib/accounts';

/** How often usage is refreshed while the window is open. Matches the core's minimum interval. */
const REFRESH_MS = 180_000;

/** Bumped after each automatic refresh; the screens reload when it changes. */
export const autoRefresh = $state({ tick: 0 });

/** Starts the once-a-minute refresh. Returns a function that stops it. */
export function startAutoRefresh(): () => void {
	const timer = setInterval(async () => {
		try {
			await refreshUsage();
		} catch {
			// The core refuses a refresh that comes too soon; keep the numbers already shown.
		}
		autoRefresh.tick += 1;
	}, REFRESH_MS);
	return () => clearInterval(timer);
}
