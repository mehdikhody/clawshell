import { invoke } from '@tauri-apps/api/core';
import type { Locale } from '$lib/locale';

/** Mirrors the core's `Settings`. The core saves them and applies the side effects. */
export type Settings = {
	/** Start clawshell when Windows starts. */
	launchAtBoot: boolean;
	/** A boot-time launch stays in the tray instead of showing the window. */
	startHidden: boolean;
	/** The close button hides to the tray; off, it quits. */
	closeToTray: boolean;
	/** The language of the window and the tray menu. */
	language: Locale;
	/** Where Claude.exe is, used when detection fails; null when not set. */
	claudePath: string | null;
	/** Notify when a window reaches the warning threshold. */
	notifyWarning: boolean;
	/** Notify when a window reaches the limit threshold. */
	notifyLimited: boolean;
	/** Notify when a window that was Limited resets. */
	notifyReset: boolean;
	/** Percent of a window at which the warning fires; below the limit threshold, 1 to 100. */
	warningThreshold: number;
	/** Percent of a window at which it counts as Limited, 1 to 100. */
	limitThreshold: number;
};

export const getSettings = () => invoke<Settings>('get_settings');

/** Saves the whole object, or rejects with a `CommandError` and saves nothing. */
export const updateSettings = (settings: Settings) =>
	invoke<Settings>('update_settings', { settings });
