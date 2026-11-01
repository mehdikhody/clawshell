import { getContext, setContext } from 'svelte';
import type { AccountList } from '$lib/accounts';
import type { MonthCost } from '$lib/cost';
import type { Heatmap } from '$lib/heatmap';

/**
 * What the screens last showed. A language or calendar change remounts the screens, so they keep
 * it here, above the remount: the new screens draw it at once instead of bringing the placeholders
 * back, and the usual reload then refreshes it.
 */
export type ScreenData = {
	usage: {
		list: AccountList | null;
		cost: MonthCost | null;
		heatmap: Heatmap | null;
		/** Whether each lazily loaded section has had its first answer, or failed. */
		listReady: boolean;
		costReady: boolean;
		heatmapReady: boolean;
	};
	accounts: { list: AccountList | null };
};

const KEY = Symbol('screen-data');

function createScreenData(): ScreenData {
	const data = $state<ScreenData>({
		usage: {
			list: null,
			cost: null,
			heatmap: null,
			listReady: false,
			costReady: false,
			heatmapReady: false
		},
		accounts: { list: null }
	});
	return data;
}

/** Called once by the shell, outside what it remounts. */
export function provideScreenData(): void {
	setContext(KEY, createScreenData());
}

/** The shell's data, or fresh data for a screen mounted on its own. */
export function useScreenData(): ScreenData {
	return getContext<ScreenData | undefined>(KEY) ?? createScreenData();
}
