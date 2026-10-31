<script lang="ts">
	import { onMount } from 'svelte';
	import IconChevronDown from '@tabler/icons-svelte/icons/chevron-down';
	import IconInfoCircle from '@tabler/icons-svelte/icons/info-circle';
	import { errorCode } from '$lib/accounts';
	import * as Alert from '$lib/components/ui/alert';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Item from '$lib/components/ui/item';
	import { Label } from '$lib/components/ui/label';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { isLocale, locales } from '$lib/locale';
	import { active, switchLocale } from '$lib/locale-state.svelte';
	import { Slider as SliderPrimitive } from 'bits-ui';
	import { Switch } from '$lib/components/ui/switch';
	import { m } from '$lib/paraglide/messages.js';
	import {
		getSettings,
		openDefaultApps,
		routingNeedsChoice,
		setSignInRouting,
		updateSettings,
		type Settings
	} from '$lib/settings';

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

	/** Routing is on but Windows sends links to another app until clawshell is picked in Default Apps. */
	let needsChoice = $state(false);
	/** How often the warning looks again, so it clears once the user has picked clawshell. */
	const CHOICE_RECHECK_MS = 3000;

	async function checkChoice() {
		try {
			needsChoice = (await routingNeedsChoice()) === true;
		} catch {
			needsChoice = false;
		}
	}

	$effect(() => {
		if (!needsChoice) return;
		const timer = setInterval(checkChoice, CHOICE_RECHECK_MS);
		return () => clearInterval(timer);
	});

	/** Routing has its own command: it also edits the registry, and the core says what is left to do. */
	async function changeRouting(enabled: boolean) {
		failure = null;
		try {
			const change = await setSignInRouting(enabled);
			settings = change.settings;
			needsChoice = enabled && change.needsDefaultAppsChoice;
		} catch (error) {
			failure = failureText[errorCode(error) ?? '']?.() ?? m.error_settings_save_failed();
		}
	}

	async function loadSettings() {
		failure = null;
		try {
			settings = await getSettings();
			draftPath = settings.claudePath ?? '';
			syncDrafts();
		} catch {
			failure = m.error_settings_load_failed();
			return;
		}
		await checkChoice();
	}

	onMount(loadSettings);

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
	let draftWarning = $state(0);
	let draftLimit = $state(0);

	function syncDrafts() {
		draftWarning = settings?.warningThreshold ?? 0;
		draftLimit = settings?.limitThreshold ?? 0;
	}

	/** Commits a threshold when the slider is released; a non-whole value goes back to what is saved. */
	async function saveThreshold(key: 'warningThreshold' | 'limitThreshold', value: number) {
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
			edit: (value: number) => (draftWarning = value)
		},
		{
			key: 'limitThreshold',
			label: () => m.settings_limit_threshold(),
			draft: () => draftLimit,
			edit: (value: number) => (draftLimit = value)
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
	<Item.Root variant="outline" size="sm">
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

<section class="flex flex-col gap-3 p-4 pb-8">
	<h1 class="text-base font-semibold">{m.tab_settings()}</h1>
	{#if failure}
		<Alert.Root variant="destructive">
			<Alert.Title>{failure}</Alert.Title>
		</Alert.Root>
	{/if}

	{#if !settings && !failure}
		<p class="py-10 text-center text-sm text-muted-foreground">{m.loading()}</p>
	{:else if !settings}
		<Button variant="outline" size="sm" class="self-center" onclick={loadSettings}>
			{m.action_retry()}
		</Button>
	{/if}

	{#if settings}
		<h2 class="text-sm font-medium">{m.settings_general()}</h2>
		{#each switches as row (row.key)}
			{@render switchRow(row)}
		{/each}

		<Item.Root variant="outline" size="sm">
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
		{#if needsChoice}
			<Alert.Root role="status">
				<IconInfoCircle />
				<Alert.Title>{m.routing_choice_title()}</Alert.Title>
				<Alert.Description>
					<p>{m.routing_choice_body()}</p>
					<Button size="sm" variant="outline" class="mt-2" onclick={() => void openDefaultApps()}>
						{m.routing_choice_open()}
					</Button>
				</Alert.Description>
			</Alert.Root>
		{/if}

		<h2 class="mt-2 text-sm font-medium">{m.settings_notifications()}</h2>
		{#each alerts as row (row.key)}
			{@render switchRow(row)}
		{/each}

		{#each thresholds as field (field.key)}
			<Item.Root variant="outline" size="sm">
				<Item.Content>
					<Item.Title>{field.label()}</Item.Title>
					<div class="flex items-center gap-3">
						<SliderPrimitive.Root
							type="single"
							min={0}
							max={100}
							step={1}
							aria-label={field.label()}
							value={field.draft()}
							onValueChange={(value) => field.edit(value)}
							onValueCommit={(value) => saveThreshold(field.key, value)}
							class="relative flex w-full touch-none items-center select-none"
						>
							<span class="relative h-1.5 w-full grow overflow-hidden rounded-full bg-muted">
								<SliderPrimitive.Range class="absolute h-full bg-primary" />
							</span>
							<SliderPrimitive.Thumb
								index={0}
								class="block size-4 shrink-0 rounded-full border border-primary bg-background shadow-sm ring-ring/50 transition-[color,box-shadow] hover:ring-4 focus-visible:ring-4 focus-visible:outline-hidden"
							/>
						</SliderPrimitive.Root>
						<span class="w-10 text-end text-sm tabular-nums">{field.draft()}%</span>
					</div>
				</Item.Content>
			</Item.Root>
		{/each}
		<p class="text-xs text-muted-foreground">{m.settings_thresholds_hint()}</p>

		<h2 class="mt-2 text-sm font-medium">{m.settings_preferences()}</h2>
		<Item.Root variant="outline" size="sm">
			<Item.Content>
				<Item.Title id="setting-language">{m.settings_language()}</Item.Title>
				<Item.Description>{m.settings_language_hint()}</Item.Description>
			</Item.Content>
			<Item.Actions>
				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Button {...props} variant="outline" size="sm" aria-labelledby="setting-language">
								{languageName[settings!.language]()}
								<IconChevronDown />
							</Button>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="end">
						<DropdownMenu.RadioGroup
							value={settings!.language}
							onValueChange={(next) => {
								if (isLocale(next) && next !== settings!.language) void save({ language: next });
							}}
						>
							{#each locales as locale (locale)}
								<DropdownMenu.RadioItem value={locale} lang={locale}
									>{languageName[locale]()}</DropdownMenu.RadioItem
								>
							{/each}
						</DropdownMenu.RadioGroup>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</Item.Actions>
		</Item.Root>

		<form onsubmit={savePath}>
			<Item.Root variant="outline" size="sm">
				<Item.Content>
					<Item.Title
						><Label for="setting-claude-path">{m.settings_claude_path()}</Label></Item.Title
					>
					<Item.Description>{m.settings_claude_path_hint()}</Item.Description>
				</Item.Content>
				<Item.Actions class="w-full">
					<Input
						id="setting-claude-path"
						class="min-w-0 flex-1"
						bind:value={draftPath}
						placeholder={m.settings_claude_path_placeholder()}
						spellcheck={false}
						autocomplete="off"
						dir="ltr"
					/>
					<Button type="submit" variant="secondary">{m.settings_claude_path_save()}</Button>
				</Item.Actions>
			</Item.Root>
		</form>
	{/if}
</section>
