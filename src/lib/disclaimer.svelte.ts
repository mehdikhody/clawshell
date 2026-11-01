import { invoke } from '@tauri-apps/api/core';

/** Mirrors the core's `DisclaimerStatus`. */
type DisclaimerStatus = { accepted: boolean; version: number };

/**
 * Whether the first-use disclaimer is accepted. Null until the core has answered, so the window
 * neither shows the dialog nor starts its background work too early.
 */
export const disclaimer = $state<{ accepted: boolean | null }>({ accepted: null });

/** Asks the core whether the disclaimer is accepted. An unreadable answer leaves it unknown. */
export async function loadDisclaimer(): Promise<void> {
	try {
		const status = await invoke<DisclaimerStatus>('disclaimer_status');
		if (typeof status?.accepted === 'boolean') disclaimer.accepted = status.accepted;
	} catch {
		// Stay unknown: nothing waiting on the disclaimer starts.
	}
}

/** Saves the acceptance in the core, which starts its deferred work. Rejects and changes nothing if it can't. */
export async function acceptDisclaimer(): Promise<void> {
	await invoke('accept_disclaimer');
	disclaimer.accepted = true;
}

/** Quits clawshell. */
export const declineDisclaimer = () => invoke<void>('decline_disclaimer');
