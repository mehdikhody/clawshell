<script lang="ts">
	import { onMount } from 'svelte';
	import IconAlertTriangle from '@tabler/icons-svelte/icons/alert-triangle';
	import IconDots from '@tabler/icons-svelte/icons/dots';
	import IconFolderOpen from '@tabler/icons-svelte/icons/folder-open';
	import IconPencil from '@tabler/icons-svelte/icons/pencil';
	import IconPlus from '@tabler/icons-svelte/icons/plus';
	import IconRefresh from '@tabler/icons-svelte/icons/refresh';
	import IconTrash from '@tabler/icons-svelte/icons/trash';
	import {
		addAccount,
		emptyPooled,
		deleteAccount,
		errorCode,
		launchAccount,
		listAccounts,
		onAccountsChanged,
		openAccountFolder,
		refreshUsage,
		renameAccount,
		type Account,
		type AccountList,
		type AccountStatus,
		type UsageSnapshot,
		type WindowUsage
	} from '$lib/accounts';
	import * as Alert from '$lib/components/ui/alert';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Badge, type BadgeVariant } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { Input } from '$lib/components/ui/input';
	import * as Item from '$lib/components/ui/item';
	import { Progress } from '$lib/components/ui/progress';
	import { m } from '$lib/paraglide/messages.js';
	import { formatTag } from '$lib/locale-state.svelte';
	import { formatPercentValue } from '$lib/locale';
	import { formatAgo, formatCountdown, formatReset } from '$lib/usage';

	let list = $state<AccountList>({
		claudeFound: true,
		refreshAllowedAt: null,
		pooled: emptyPooled,
		accounts: []
	});
	let loaded = $state(false);
	let adding = $state(false);
	let refreshing = $state(false);
	/** Ticks every second so "updated X ago" and the refresh wait stay current. */
	let now = $state(Date.now());
	/** How long until a manual refresh is allowed again; 0 when it is allowed now. */
	const refreshWaitMs = $derived(Math.max(0, (list.refreshAllowedAt ?? 0) - now));
	let failure = $state<string | null>(null);

	/** The account being renamed, and the draft name in the dialog. */
	let renaming = $state<Account | null>(null);
	let draftName = $state('');
	/** The account awaiting delete (or, when its folder is gone, remove) confirmation. */
	let confirming = $state<Account | null>(null);

	const statusLabel: Record<AccountStatus, () => string> = {
		running: () => m.status_running(),
		idle: () => m.status_idle(),
		'signed-out': () => m.status_signed_out(),
		limited: () => m.status_limited(),
		missing: () => m.status_missing()
	};
	const statusVariant: Record<AccountStatus, BadgeVariant> = {
		running: 'default',
		idle: 'secondary',
		'signed-out': 'destructive',
		limited: 'outline',
		missing: 'destructive'
	};

	async function refresh() {
		try {
			list = await listAccounts();
			loaded = true;
		} catch {
			// The core pushes the next snapshot when it can.
		}
	}

	async function refreshNow() {
		refreshing = true;
		failure = null;
		try {
			const outcome = await refreshUsage();
			list = { ...list, refreshAllowedAt: outcome.refreshAllowedAt };
		} catch {
			failure = m.error_refresh_failed();
		} finally {
			refreshing = false;
			await refresh();
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

	async function launch(account: Account) {
		if (account.status === 'missing') return;
		failure = null;
		try {
			await launchAccount(account.id);
		} catch (error) {
			failure = describe(error, m.error_launch_failed());
		}
	}

	function startRename(account: Account) {
		draftName = account.label;
		renaming = account;
	}

	async function saveRename() {
		const account = renaming;
		const label = draftName.trim();
		if (!account || !label) return;
		renaming = null;
		failure = null;
		try {
			await renameAccount(account.id, label);
		} catch (error) {
			failure = describe(error, m.error_rename_failed());
		} finally {
			await refresh();
		}
	}

	async function openFolder(account: Account) {
		failure = null;
		try {
			await openAccountFolder(account.id);
		} catch (error) {
			failure = describe(error, m.error_open_folder_failed());
		}
	}

	function askDelete(account: Account) {
		if (account.running) {
			failure = m.error_delete_running({ label: account.label });
			return;
		}
		failure = null;
		confirming = account;
	}

	async function confirmDelete() {
		const account = confirming;
		if (!account) return;
		confirming = null;
		failure = null;
		try {
			await deleteAccount(account.id);
		} catch (error) {
			failure =
				errorCode(error) === 'account-running'
					? m.error_delete_running({ label: account.label })
					: describe(error, m.error_delete_failed());
		} finally {
			await refresh();
		}
	}

	onMount(() => {
		const tick = setInterval(() => (now = Date.now()), 1000);
		void refresh();
		const unlisten = onAccountsChanged((next) => {
			list = next;
			loaded = true;
		});
		return () => {
			clearInterval(tick);
			void unlisten.then((stop) => stop()).catch(() => {});
		};
	});
</script>

<section class="flex flex-col gap-3 p-4">
	<header class="flex items-center justify-between">
		<h1 class="text-base font-semibold">{m.accounts_title()}</h1>
		<div class="flex items-center gap-2">
			<Button
				size="icon-sm"
				variant="outline"
				aria-label={m.refresh_usage()}
				onclick={refreshNow}
				disabled={refreshing || refreshWaitMs > 0}
			>
				<IconRefresh class={refreshing ? 'animate-spin' : ''} />
			</Button>
			<Button size="sm" onclick={add} disabled={adding || !list.claudeFound}>
				<IconPlus />
				{m.accounts_add()}
			</Button>
		</div>
	</header>

	{#if refreshWaitMs > 0}
		<p class="text-xs text-muted-foreground">
			{m.refresh_next({ time: formatCountdown(refreshWaitMs, formatTag()) })}
		</p>
	{/if}

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
					ondblclick={() => launch(account)}
					onkeydown={(event: KeyboardEvent) => {
						// Keys pressed on the row's own buttons belong to those buttons.
						if (event.key === 'Enter' && event.target === event.currentTarget) {
							void launch(account);
						}
					}}
				>
					<Item.Content>
						<Item.Title>{account.label}</Item.Title>
						{#if account.status === 'missing'}
							<Item.Description>{m.missing_hint()}</Item.Description>
						{:else}
							{#if account.email}
								<Item.Description>{account.email}</Item.Description>
							{/if}
							{#if account.usage}
								{@render usageLines(account.usage)}
							{:else if account.status !== 'signed-out'}
								<p class="text-xs text-muted-foreground">{m.usage_none()}</p>
							{/if}
						{/if}
					</Item.Content>
					<Item.Actions>
						{#if account.status === 'limited' && account.running}
							<Badge variant="default">{statusLabel.running()}</Badge>
						{/if}
						<Badge variant={statusVariant[account.status]}>{statusLabel[account.status]()}</Badge>
						{#if account.status === 'missing'}
							<Button size="xs" variant="destructive" onclick={() => askDelete(account)}>
								{m.missing_remove()}
							</Button>
						{/if}
						<DropdownMenu.Root>
							<DropdownMenu.Trigger>
								{#snippet child({ props })}
									<Button
										{...props}
										variant="ghost"
										size="icon-sm"
										aria-label={m.accounts_actions({ label: account.label })}
										ondblclick={(event: MouseEvent) => event.stopPropagation()}
									>
										<IconDots />
									</Button>
								{/snippet}
							</DropdownMenu.Trigger>
							<DropdownMenu.Content align="end">
								<DropdownMenu.Item onSelect={() => startRename(account)}>
									<IconPencil />
									{m.menu_rename()}
								</DropdownMenu.Item>
								<DropdownMenu.Item
									disabled={account.status === 'missing'}
									onSelect={() => openFolder(account)}
								>
									<IconFolderOpen />
									{m.menu_open_folder()}
								</DropdownMenu.Item>
								<DropdownMenu.Separator />
								<DropdownMenu.Item variant="destructive" onSelect={() => askDelete(account)}>
									<IconTrash />
									{m.menu_delete()}
								</DropdownMenu.Item>
							</DropdownMenu.Content>
						</DropdownMenu.Root>
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

{#snippet usageWindow(label: string, window: WindowUsage | null)}
	{#if window}
		<div class="flex items-center gap-2 text-xs">
			<span class="w-14 shrink-0 text-muted-foreground">{label}</span>
			<Progress
				value={window.utilization}
				aria-label={label}
				class={window.utilization >= 100 ? '*:data-[slot=progress-indicator]:bg-destructive' : ''}
			/>
			<span class="w-9 shrink-0 text-end tabular-nums"
				>{formatPercentValue(Math.round(window.utilization), formatTag())}</span
			>
		</div>
		{#if window.resetsAt}
			<p class="ps-16 text-[11px] text-muted-foreground">
				{m.usage_resets({ time: formatReset(window.resetsAt, new Date(now), formatTag()) })}
			</p>
		{/if}
	{/if}
{/snippet}

{#snippet usageLines(usage: UsageSnapshot)}
	{@render usageWindow(m.usage_five_hour(), usage.fiveHour)}
	{@render usageWindow(m.usage_weekly(), usage.weekly)}
	{#if usage.source !== 'live'}
		{@const ago = formatAgo(usage.fetchedAtMs, now, formatTag())}
		<p class="text-[11px] text-muted-foreground">
			{usage.source === 'stale' ? m.usage_stale({ ago }) : m.usage_local_history({ ago })}
		</p>
	{/if}
{/snippet}

<Dialog.Root
	open={renaming !== null}
	onOpenChange={(open) => {
		if (!open) renaming = null;
	}}
>
	<Dialog.Content class="sm:max-w-sm">
		<form
			class="flex flex-col gap-4"
			onsubmit={(event) => {
				event.preventDefault();
				void saveRename();
			}}
		>
			<Dialog.Header>
				<Dialog.Title>{m.rename_title()}</Dialog.Title>
			</Dialog.Header>
			<Input aria-label={m.rename_field()} bind:value={draftName} autocomplete="off" />
			<Dialog.Footer>
				<Dialog.Close type="button">
					{#snippet child({ props })}
						<Button {...props} variant="outline">{m.action_cancel()}</Button>
					{/snippet}
				</Dialog.Close>
				<Button type="submit" disabled={draftName.trim() === ''}>{m.rename_save()}</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>

<AlertDialog.Root
	open={confirming !== null}
	onOpenChange={(open) => {
		if (!open) confirming = null;
	}}
>
	<AlertDialog.Content>
		<AlertDialog.Header>
			{#if confirming?.status === 'missing'}
				<AlertDialog.Title>{m.missing_title({ label: confirming.label })}</AlertDialog.Title>
				<AlertDialog.Description>{m.missing_body()}</AlertDialog.Description>
			{:else if confirming}
				<AlertDialog.Title>{m.delete_title({ label: confirming.label })}</AlertDialog.Title>
				<AlertDialog.Description>{m.delete_body()}</AlertDialog.Description>
			{/if}
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>{m.action_cancel()}</AlertDialog.Cancel>
			<AlertDialog.Action variant="destructive" onclick={confirmDelete}>
				{confirming?.status === 'missing' ? m.missing_confirm() : m.delete_confirm()}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
