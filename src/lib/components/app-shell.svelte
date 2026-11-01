<script lang="ts">
	import { onMount } from 'svelte';
	import IconChartBar from '@tabler/icons-svelte/icons/chart-bar';
	import IconSettings from '@tabler/icons-svelte/icons/settings';
	import IconUsers from '@tabler/icons-svelte/icons/users';
	import * as Tabs from '$lib/components/ui/tabs';
	import { isCalendar, isLocale, type CalendarSetting, type Locale } from '$lib/locale';
	import { active, switchCalendar, switchLocale } from '$lib/locale-state.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import AccountsScreen from '$lib/screens/accounts-screen.svelte';
	import SettingsScreen from '$lib/screens/settings-screen.svelte';
	import UsageScreen from '$lib/screens/usage-screen.svelte';
	import { getSettings } from '$lib/settings';
	import { onAddAccountRequested } from '$lib/accounts';
	import { addRequest } from '$lib/add-account-request.svelte';
	import { startAutoRefresh } from '$lib/auto-refresh.svelte';
	import { startUpdateChecks } from '$lib/update.svelte';
	import RoutingOffer from './routing-offer.svelte';
	import TopBar from './top-bar.svelte';
	import { scrollFade } from '$lib/scroll-fade';

	let tab = $state('accounts');
	/** Nothing is drawn until the saved language is known, so the window never flashes the wrong one. */
	let ready = $state(false);
	let usageEl = $state<HTMLElement | null>(null);
	let accountsEl = $state<HTMLElement | null>(null);
	let settingsEl = $state<HTMLElement | null>(null);

	$effect(() => {
		const cleanups = [usageEl, accountsEl, settingsEl]
			.filter((el): el is HTMLElement => el !== null)
			.map(scrollFade);
		return () => cleanups.forEach((cleanup) => cleanup());
	});

	let stopUpdateChecks: (() => void) | null = null;
	let unmounted = false;

	// onMount's own callback is async, so the cleanup is registered separately.
	onMount(() => () => {
		unmounted = true;
		stopUpdateChecks?.();
	});

	onMount(() => {
		// The tray's Add account opens the dialog on the Accounts tab.
		const unlisten = onAddAccountRequested(() => {
			tab = 'accounts';
			addRequest.pending = true;
		});
		return () => void unlisten.then((stop) => stop()).catch(() => {});
	});

	onMount(async () => {
		let language: Locale = 'en';
		let calendar: CalendarSetting = 'automatic';
		try {
			const saved = await getSettings();
			// Without readable settings the window opens in English.
			if (isLocale(saved?.language)) language = saved.language;
			if (isCalendar(saved?.calendar)) calendar = saved.calendar;
		} catch {
			// Keep English.
		}
		// Always applied: the built page carries unfilled lang and dir placeholders.
		switchLocale(language);
		switchCalendar(calendar);
		startAutoRefresh();
		if (!unmounted) stopUpdateChecks = startUpdateChecks();
		ready = true;
	});
</script>

{#if ready}
	<!-- Messages are plain functions, so a new language or calendar redraws everything by re-keying. `tab` lives above the key and survives. -->
	{#key `${active.locale}-${active.calendar}`}
		<div class="flex h-screen w-screen flex-col overflow-hidden rounded-xl border bg-background">
			<TopBar />
			<Tabs.Root bind:value={tab} class="min-h-0 flex-1 gap-0 data-horizontal:flex-col-reverse">
				<Tabs.List variant="line" class="group-data-horizontal/tabs:h-14 w-full shrink-0 rounded-none border-t p-0">
					<Tabs.Trigger value="usage" class="h-full flex-col gap-1 text-xs group-data-horizontal/tabs:after:top-0 group-data-horizontal/tabs:after:bottom-auto">
						<IconChartBar />
						{m.tab_usage()}
					</Tabs.Trigger>
					<Tabs.Trigger value="accounts" class="h-full flex-col gap-1 text-xs group-data-horizontal/tabs:after:top-0 group-data-horizontal/tabs:after:bottom-auto">
						<IconUsers />
						{m.tab_accounts()}
					</Tabs.Trigger>
					<Tabs.Trigger value="settings" class="h-full flex-col gap-1 text-xs group-data-horizontal/tabs:after:top-0 group-data-horizontal/tabs:after:bottom-auto">
						<IconSettings />
						{m.tab_settings()}
					</Tabs.Trigger>
				</Tabs.List>
				<Tabs.Content
					value="usage"
					aria-label={m.tab_usage()}
					class="min-h-0 overflow-y-auto"
					bind:ref={usageEl}
				>
					<UsageScreen />
				</Tabs.Content>
				<Tabs.Content
					value="accounts"
					bind:ref={accountsEl}
					aria-label={m.tab_accounts()}
					class="min-h-0 overflow-y-auto"
				>
					<AccountsScreen />
				</Tabs.Content>
				<Tabs.Content
					value="settings"
					bind:ref={settingsEl}
					aria-label={m.tab_settings()}
					class="min-h-0 overflow-y-auto"
				>
					<SettingsScreen />
				</Tabs.Content>
			</Tabs.Root>
		</div>
		<RoutingOffer />
	{/key}
{/if}
