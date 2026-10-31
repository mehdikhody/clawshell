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
};

export const getSettings = () => invoke<Settings>('get_settings');

/** Saves the whole object, or rejects with a `CommandError` and saves nothing. */
export const updateSettings = (settings: Settings) =>
	invoke<Settings>('update_settings', { settings });
