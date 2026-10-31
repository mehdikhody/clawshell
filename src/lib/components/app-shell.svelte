<script lang="ts">
	import IconChartBar from '@tabler/icons-svelte/icons/chart-bar';
	import IconSettings from '@tabler/icons-svelte/icons/settings';
	import IconUsers from '@tabler/icons-svelte/icons/users';
	import * as Tabs from '$lib/components/ui/tabs';
	import { m } from '$lib/paraglide/messages.js';
	import AccountsScreen from '$lib/screens/accounts-screen.svelte';
	import SettingsScreen from '$lib/screens/settings-screen.svelte';
	import UsageScreen from '$lib/screens/usage-screen.svelte';
	import TopBar from './top-bar.svelte';

	let tab = $state('usage');
</script>

<div class="flex h-screen w-screen flex-col overflow-hidden rounded-xl border bg-background">
	<TopBar />
	<Tabs.Root bind:value={tab} class="min-h-0 flex-1 gap-0 data-horizontal:flex-col-reverse">
		<Tabs.List variant="line" class="h-14 w-full shrink-0 rounded-none border-t p-0">
			<Tabs.Trigger value="usage" class="h-full flex-col gap-0.5 text-xs">
				<IconChartBar />
				{m.tab_usage()}
			</Tabs.Trigger>
			<Tabs.Trigger value="accounts" class="h-full flex-col gap-0.5 text-xs">
				<IconUsers />
				{m.tab_accounts()}
			</Tabs.Trigger>
			<Tabs.Trigger value="settings" class="h-full flex-col gap-0.5 text-xs">
				<IconSettings />
				{m.tab_settings()}
			</Tabs.Trigger>
		</Tabs.List>
		<Tabs.Content value="usage" aria-label={m.tab_usage()} class="min-h-0 overflow-y-auto">
			<UsageScreen />
		</Tabs.Content>
		<Tabs.Content value="accounts" aria-label={m.tab_accounts()} class="min-h-0 overflow-y-auto">
			<AccountsScreen />
		</Tabs.Content>
		<Tabs.Content value="settings" aria-label={m.tab_settings()} class="min-h-0 overflow-y-auto">
			<SettingsScreen />
		</Tabs.Content>
	</Tabs.Root>
</div>
