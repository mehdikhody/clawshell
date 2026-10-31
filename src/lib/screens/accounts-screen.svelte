<script lang="ts">
	import { onMount } from 'svelte';
	import IconAlertTriangle from '@tabler/icons-svelte/icons/alert-triangle';
	import IconPlus from '@tabler/icons-svelte/icons/plus';
	import {
		addAccount,
		errorCode,
		launchAccount,
		listAccounts,
		onAccountsChanged,
		type AccountList,
		type AccountStatus
	} from '$lib/accounts';
	import * as Alert from '$lib/components/ui/alert';
	import { Badge, type BadgeVariant } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Item from '$lib/components/ui/item';
	import { m } from '$lib/paraglide/messages.js';

	let list = $state<AccountList>({ claudeFound: true, accounts: [] });
	let loaded = $state(false);
	let adding = $state(false);
	let failure = $state<string | null>(null);

	const statusLabel: Record<AccountStatus, () => string> = {
		running: () => m.status_running(),
		idle: () => m.status_idle(),
		'signed-out': () => m.status_signed_out()
	};
	const statusVariant: Record<AccountStatus, BadgeVariant> = {
		running: 'default',
		idle: 'secondary',
		'signed-out': 'destructive'
	};

	async function refresh() {
		try {
			list = await listAccounts();
			loaded = true;
		} catch {
			// The core pushes the next snapshot when it can.
		}
	}

	function describe(error: unknown, fallback: string) {
		return errorCode(error) === 'claude-not-found' ? m.claude_missing_title() : fallback;
	}

	async function add() {
		adding = true;
		failure = null;
		try {
			await addAccount();
		} catch (error) {
			failure = describe(error, m.error_add_failed());
		} finally {
			adding = false;
			await refresh();
		}
	}

	async function launch(id: string) {
		failure = null;
		try {
			await launchAccount(id);
		} catch (error) {
			failure = describe(error, m.error_launch_failed());
		}
	}

	onMount(() => {
		void refresh();
		const unlisten = onAccountsChanged((next) => {
			list = next;
			loaded = true;
		});
		return () => {
			void unlisten.then((stop) => stop()).catch(() => {});
		};
	});
</script>

<section class="flex flex-col gap-3 p-4">
	<header class="flex items-center justify-between">
		<h1 class="text-base font-semibold">{m.accounts_title()}</h1>
		<Button size="sm" onclick={add} disabled={adding || !list.claudeFound}>
			<IconPlus />
			{m.accounts_add()}
		</Button>
	</header>

	{#if !list.claudeFound}
		<Alert.Root variant="destructive">
			<IconAlertTriangle />
			<Alert.Title>{m.claude_missing_title()}</Alert.Title>
			<Alert.Description>{m.claude_missing_body()}</Alert.Description>
		</Alert.Root>
	{/if}

	{#if failure}
		<Alert.Root variant="destructive">
			<IconAlertTriangle />
			<Alert.Title>{failure}</Alert.Title>
		</Alert.Root>
	{/if}

	{#if list.accounts.length > 0}
		<Item.Group class="gap-2">
			{#each list.accounts as account (account.id)}
				<Item.Root
					role="listitem"
					variant="outline"
					size="sm"
					tabindex={0}
					class="cursor-default select-none"
					ondblclick={() => launch(account.id)}
					onkeydown={(event: KeyboardEvent) => {
						if (event.key === 'Enter') void launch(account.id);
					}}
				>
					<Item.Content>
						<Item.Title>{account.label}</Item.Title>
					</Item.Content>
					<Item.Actions>
						<Badge variant={statusVariant[account.status]}>{statusLabel[account.status]()}</Badge>
					</Item.Actions>
				</Item.Root>
			{/each}
		</Item.Group>
		<p class="text-xs text-muted-foreground">{m.accounts_hint()}</p>
	{:else if loaded && list.claudeFound}
		<div class="flex flex-col items-center gap-1 py-10 text-center">
			<p class="text-sm font-medium">{m.accounts_empty()}</p>
			<p class="text-xs text-muted-foreground">{m.accounts_empty_hint()}</p>
		</div>
	{/if}
</section>
