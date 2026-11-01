import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CalendarSetting, Locale } from '$lib/locale';

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
	/** How dates are written, which digits numbers use and which day weeks start on; Automatic follows the language. */
	calendar: CalendarSetting;
	/** Where Claude.exe is, used when detection fails; null when not set. */
	claudePath: string | null;
	/** Notify when a window reaches the warning threshold. */
	notifyWarning: boolean;
	/** Notify when a window reaches the limit threshold. */
	notifyLimited: boolean;
	/** Notify when a window (5-hour or weekly) resets. */
	notifyReset: boolean;
	/** Percent of a window at which the warning fires; below the limit threshold, 1 to 100. */
	warningThreshold: number;
	/** Percent of a window at which it counts as Limited, 1 to 100. */
	limitThreshold: number;
	/** Browser sign-in links reach the account that asked for them. Change it with `setSignInRouting`. */
	signInRouting: boolean;
	/** The disclaimer version the user accepted; null until they accept. The core changes it only on acceptance. */
	disclaimerAccepted: number | null;
};

export const getSettings = () => invoke<Settings>('get_settings');

/** Saves the whole object, or rejects with a `CommandError` and saves nothing. */
export const updateSettings = (settings: Settings) =>
	invoke<Settings>('update_settings', { settings });

/** What turning routing on or off changed. */
export type RoutingChange = {
	settings: Settings;
	/** The Store build: the user still has to pick clawshell on the Default Apps page, which is open. */
	needsDefaultAppsChoice: boolean;
};

/**
 * Turns sign-in routing on or off. Routing has its own command because it also edits the
 * registry; `updateSettings` leaves the field as the core has it. Rejects with
 * a `CommandError` (`routing-failed`) and changes nothing.
 */
export const setSignInRouting = (enabled: boolean) =>
	invoke<RoutingChange>('set_sign_in_routing', { enabled });

/**
 * Routing is on, but Windows sends `claude://` links to another app (Claude itself, on the
 * Store build): the user has yet to pick clawshell on the Default Apps page.
 */
export const routingNeedsChoice = () => invoke<boolean>('routing_needs_choice');

/** Opens the Windows Default Apps page for clawshell. */
export const openDefaultApps = () => invoke<void>('open_default_apps');

/** Calls `handler` once, after the first successful Add, when routing should be offered. */
export const onRoutingOffer = (handler: () => void): Promise<UnlistenFn> =>
	listen('routing-offer', () => handler());
