<script lang="ts">
	import { onMount } from 'svelte';
	import IconInfoCircle from '@tabler/icons-svelte/icons/info-circle';
	import { errorCode } from '$lib/accounts';
	import * as Alert from '$lib/components/ui/alert';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Item from '$lib/components/ui/item';
	import { Label } from '$lib/components/ui/label';
	import * as ToggleGroup from '$lib/components/ui/toggle-group';
	import { isLocale, locales } from '$lib/locale';
	import { active, switchLocale } from '$lib/locale-state.svelte';
	import { Switch } from '$lib/components/ui/switch';
	import { m } from '$lib/paraglide/messages.js';
	import { getSettings, setSignInRouting, updateSettings, type Settings } from '$lib/settings';

	/** What the core has saved. It changes only when the core accepts an update. */
	let settings = $state<Settings | null>(null);
	/** The Claude.exe field's text, which may be ahead of what is saved. */
	let draftPath = $state('');
	let failure = $state<string | null>(null);

	const failureText: Record<string, () => string> = {
		'autostart-failed': () => m.error_settings_autostart(),
		'claude-path-missing': () => m.error_claude_path_missing(),
		'claude-path-not-a-file': () => m.error_claude_path_not_a_file(),
		'claude-path-not-claude': () => m.error_claude_path_not_claude(),
		'invalid-thresholds': () => m.error_thresholds_invalid(),
		'routing-failed': () => m.error_routing_failed()
	};

	/** The Store build leaves one choice on the Default Apps page, which has just been opened. */
	let defaultAppsNote = $state(false);

	/** Routing has its own command: it also edits the registry, and the core says what is left to do. */
	async function changeRouting(enabled: boolean) {
		failure = null;
		try {
			const change = await setSignInRouting(enabled);
			settings = change.settings;
			defaultAppsNote = enabled && change.needsDefaultAppsChoice;
		} catch (error) {
			failure = failureText[errorCode(error) ?? '']?.() ?? m.error_settings_save_failed();
		}
	}

	onMount(async () => {
		try {
			settings = await getSettings();
			draftPath = settings.claudePath ?? '';
			syncDrafts();
		} catch {
			failure = m.error_settings_save_failed();
		}
	});

	/** Sends the whole object; the screen follows the core's answer, never its own guess. */
	async function save(change: Partial<Settings>) {
		if (!settings) return;
		failure = null;
		try {
			settings = await updateSettings({ ...settings, ...change });
			draftPath = settings.claudePath ?? '';
			syncDrafts();
			if (settings.language !== active.locale) switchLocale(settings.language);
		} catch (error) {
			failure = failureText[errorCode(error) ?? '']?.() ?? m.error_settings_save_failed();
		}
	}

	function savePath(event: SubmitEvent) {
		event.preventDefault();
		const path = draftPath.trim();
		return save({ claudePath: path === '' ? null : path });
	}

	/** The threshold fields' text, which may be ahead of what is saved. */
	let draftWarning = $state('');
	let draftLimit = $state('');

	function syncDrafts() {
		draftWarning = String(settings?.warningThreshold ?? '');
		draftLimit = String(settings?.limitThreshold ?? '');
	}

	/** Commits a threshold field; an empty or non-whole value goes back to what is saved. */
	async function saveThreshold(key: 'warningThreshold' | 'limitThreshold', text: string) {
		const value = text.trim() === '' ? NaN : Number(text);
		if (Number.isInteger(value) && value !== settings?.[key]) await save({ [key]: value });
		syncDrafts();
	}

	const languageName = { en: () => m.language_en(), fa: () => m.language_fa() };

	const switches = [
		{
			key: 'launchAtBoot',
			title: () => m.settings_launch_at_boot(),
			hint: () => m.settings_launch_at_boot_hint()
		},
		{
			key: 'startHidden',
			title: () => m.settings_start_hidden(),
			hint: () => m.settings_start_hidden_hint()
		},
		{
			key: 'closeToTray',
			title: () => m.settings_close_to_tray(),
			hint: () => m.settings_close_to_tray_hint()
		}
	] as const;

	const alerts = [
		{
			key: 'notifyWarning',
			title: () => m.settings_notify_warning(),
			hint: () => m.settings_notify_warning_hint()
		},
		{
			key: 'notifyLimited',
			title: () => m.settings_notify_limited(),
			hint: () => m.settings_notify_limited_hint()
		},
		{
			key: 'notifyReset',
			title: () => m.settings_notify_reset(),
			hint: () => m.settings_notify_reset_hint()
		}
	] as const;

	const thresholds = [
		{
			key: 'warningThreshold',
			label: () => m.settings_warning_threshold(),
			draft: () => draftWarning,
			edit: (value: string) => (draftWarning = value)
		},
		{
			key: 'limitThreshold',
			label: () => m.settings_limit_threshold(),
			draft: () => draftLimit,
			edit: (value: string) => (draftLimit = value)
		}
	] as const;
</script>

{#snippet switchRow(row: {
	key:
		| 'launchAtBoot'
		| 'startHidden'
		| 'closeToTray'
		| 'notifyWarning'
		| 'notifyLimited'
		| 'notifyReset';
	title: () => string;
	hint: () => string;
})}
	<Item.Root variant="outline">
		<Item.Content>
			<Item.Title><Label for="setting-{row.key}">{row.title()}</Label></Item.Title>
			<Item.Description>{row.hint()}</Item.Description>
		</Item.Content>
		<Item.Actions>
			<Switch
				id="setting-{row.key}"
				bind:checked={() => settings![row.key], (checked) => void save({ [row.key]: checked })}
			/>
		</Item.Actions>
	</Item.Root>
{/snippet}

<section class="flex flex-col gap-3 p-4">
	{#if failure}
		<Alert.Root variant="destructive">
			<Alert.Title>{failure}</Alert.Title>
		</Alert.Root>
	{/if}

	{#if settings}
		{#each switches as row (row.key)}
			{@render switchRow(row)}
		{/each}

		<Item.Root variant="outline">
			<Item.Content>
				<Item.Title
					><Label for="setting-signInRouting">{m.settings_sign_in_routing()}</Label></Item.Title
				>
				<Item.Description>{m.settings_sign_in_routing_hint()}</Item.Description>
			</Item.Content>
			<Item.Actions>
				<Switch
					id="setting-signInRouting"
					bind:checked={() => settings!.signInRouting, (checked) => void changeRouting(checked)}
				/>
			</Item.Actions>
		</Item.Root>
		{#if defaultAppsNote}
			<Alert.Root role="status">
				<IconInfoCircle />
				<Alert.Title>{m.routing_default_apps_title()}</Alert.Title>
				<Alert.Description>{m.routing_default_apps_body()}</Alert.Description>
			</Alert.Root>
		{/if}

		<h2 class="mt-2 text-sm font-medium">{m.settings_notifications()}</h2>
		{#each alerts as row (row.key)}
			{@render switchRow(row)}
		{/each}

		<div class="flex flex-col gap-2">
			<div class="flex gap-2">
				{#each thresholds as field (field.key)}
					<div class="flex flex-1 flex-col gap-2">
						<Label for="setting-{field.key}">{field.label()}</Label>
						<Input
							id="setting-{field.key}"
							type="number"
							min={1}
							max={100}
							step={1}
							inputmode="numeric"
							dir="ltr"
							value={field.draft()}
							oninput={(event) => field.edit(event.currentTarget.value)}
							onchange={() => saveThreshold(field.key, field.draft())}
						/>
					</div>
				{/each}
			</div>
			<p class="text-xs text-muted-foreground">{m.settings_thresholds_hint()}</p>
		</div>

		<Item.Root variant="outline">
			<Item.Content>
				<Item.Title id="setting-language">{m.settings_language()}</Item.Title>
				<Item.Description>{m.settings_language_hint()}</Item.Description>
			</Item.Content>
			<Item.Actions>
				<ToggleGroup.Root
					type="single"
					variant="outline"
					size="sm"
					aria-labelledby="setting-language"
					bind:value={
						() => settings!.language,
						(next) => {
							// A single-select group reports '' when the pressed item is pressed again.
							if (isLocale(next) && next !== settings!.language) void save({ language: next });
						}
					}
				>
					{#each locales as locale (locale)}
						<ToggleGroup.Item value={locale} lang={locale}
							>{languageName[locale]()}</ToggleGroup.Item
						>
					{/each}
				</ToggleGroup.Root>
			</Item.Actions>
		</Item.Root>

		<form class="flex flex-col gap-2" onsubmit={savePath}>
			<Label for="setting-claude-path">{m.settings_claude_path()}</Label>
			<div class="flex gap-2">
				<Input
					id="setting-claude-path"
					bind:value={draftPath}
					placeholder={m.settings_claude_path_placeholder()}
					spellcheck={false}
					autocomplete="off"
					dir="ltr"
				/>
				<Button type="submit" variant="secondary">{m.settings_claude_path_save()}</Button>
			</div>
			<p class="text-xs text-muted-foreground">{m.settings_claude_path_hint()}</p>
		</form>
	{/if}
</section>
