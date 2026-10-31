<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { autoRefresh } from '$lib/auto-refresh.svelte';
	import IconAlertTriangle from '@tabler/icons-svelte/icons/alert-triangle';
	import IconAppWindow from '@tabler/icons-svelte/icons/app-window';
	import IconCheck from '@tabler/icons-svelte/icons/check';
	import IconDots from '@tabler/icons-svelte/icons/dots';
	import IconFolderOpen from '@tabler/icons-svelte/icons/folder-open';
	import IconPalette from '@tabler/icons-svelte/icons/palette';
	import IconPencil from '@tabler/icons-svelte/icons/pencil';
	import IconPlus from '@tabler/icons-svelte/icons/plus';
		import IconTerminal2 from '@tabler/icons-svelte/icons/terminal-2';
	import IconTrash from '@tabler/icons-svelte/icons/trash';
	import {
		addAccount,
		addShortcuts,
		emptyPooled,
		deleteAccount,
		errorCode,
		launchAccount,
		listAccounts,
		onAccountsChanged,
		openAccountFolder,
		renameAccount,
		setAccountColour,
		type Account,
		type AccountList,
		type AccountStatus,
		type UsageSnapshot,
		type WindowUsage
	} from '$lib/accounts';
	import * as Alert from '$lib/components/ui/alert';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import AccountIcon from '$lib/components/account-icon.svelte';
	import LaunchOptionsDialog from '$lib/components/launch-options-dialog.svelte';
	import { Badge, type BadgeVariant } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import * as ContextMenu from '$lib/components/ui/context-menu';
	import { Input } from '$lib/components/ui/input';
	import * as Item from '$lib/components/ui/item';
	import { Progress } from '$lib/components/ui/progress';
	import { m } from '$lib/paraglide/messages.js';
	import { formatTag } from '$lib/locale-state.svelte';
	import { formatPercentValue } from '$lib/locale';
	import { formatAgo, formatReset } from '$lib/usage';

	let list = $state<AccountList>({
		claudeFound: true,
		refreshAllowedAt: null,
		pooled: emptyPooled,
		palette: [],
		accounts: []
	});
	/** How long a confirmation stays on screen. */
	const NOTICE_MS = 5000;
	let loaded = $state(false);
	let adding = $state(false);
		/** Ticks every second so "updated X ago" and the refresh wait stay current. */
	let now = $state(Date.now());
	let failure = $state<string | null>(null);
	/** A short confirmation for an action that otherwise shows nothing. */
	let notice = $state<string | null>(null);

	/** The account being renamed, and the draft name in the dialog. */
	let renaming = $state<Account | null>(null);
	let draftName = $state('');
	/** The account whose badge colour is being picked. */
	let colouring = $state<Account | null>(null);
	/** The account whose environment variables and flags are being edited. */
	let editingOptions = $state<Account | null>(null);
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
		running: 'secondary',
		idle: 'outline',
		'signed-out': 'outline',
		limited: 'destructive',
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

	async function changeColour(account: Account, colour: string) {
		colouring = null;
		failure = null;
		try {
			await setAccountColour(account.id, colour);
		} catch (error) {
			failure = describe(error, m.error_colour_failed());
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

	async function createShortcuts(account: Account) {
		failure = null;
		notice = null;
		try {
			await addShortcuts(account.id);
			const shown = (notice = m.shortcuts_added());
			setTimeout(() => {
				if (notice === shown) notice = null;
			}, NOTICE_MS);
		} catch (error) {
			failure = describe(error, m.error_shortcuts_failed());
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

	$effect(() => {
		void autoRefresh.tick;
		untrack(() => void refresh());
	});

	onMount(() => {
		const tick = setInterval(() => (now = Date.now()), 1000);
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
			<Button size="sm" onclick={add} disabled={adding || !list.claudeFound}>
				<IconPlus />
				{m.accounts_add()}
			</Button>
		</div>
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

	{#if notice}
		<Alert.Root role="status">
			<IconCheck />
			<Alert.Title>{notice}</Alert.Title>
		</Alert.Root>
	{/if}

	{#if list.accounts.length > 0}
		<Item.Group class="gap-2">
			{#each list.accounts as account (account.id)}
				<ContextMenu.Root>
<ContextMenu.Trigger class="contents">
<Item.Root
					role="listitem"
					variant="outline"
					size="sm"
					tabindex={0}
					class="cursor-default flex-wrap select-none"
					ondblclick={() => launch(account)}
					onkeydown={(event: KeyboardEvent) => {
						// Keys pressed on the row's own buttons belong to those buttons.
						if (event.key === 'Enter' && event.target === event.currentTarget) {
							void launch(account);
						}
					}}
				>
					<Item.Media>
						<AccountIcon badge={account.badge} colour={account.colour} />
					</Item.Media>
					<Item.Content>
						<Item.Title>{account.label}</Item.Title>
						{#if account.status === 'missing'}
							<Item.Description>{m.missing_hint()}</Item.Description>
						{:else}
							{#if account.email}
								<Item.Description class="text-xs">{account.email}</Item.Description>
							{/if}
							{#if !account.usage && account.status !== 'signed-out'}
								<p class="text-xs text-muted-foreground">{m.usage_none()}</p>
							{/if}
						{/if}
					</Item.Content>
					<Item.Actions class="self-start">
						{#if account.status === 'limited' && account.running}
							<Badge variant="secondary">{statusLabel.running()}</Badge>
						{/if}
						<Badge variant={statusVariant[account.status]}>{statusLabel[account.status]()}</Badge>
						{#if account.status === 'missing'}
							<Button size="xs" variant="destructive" onclick={() => askDelete(account)}>
								{m.missing_remove()}
							</Button>
						{/if}
						
					</Item.Actions>
					{#if account.usage}
						<div class="flex w-full flex-col gap-3 text-xs">
							{@render usageLines(account.usage)}
						</div>
					{/if}
				</Item.Root>
</ContextMenu.Trigger>
<ContextMenu.Content class="w-auto min-w-48">
								<ContextMenu.Item onSelect={() => startRename(account)}>
									<IconPencil />
									{m.menu_rename()}
								</ContextMenu.Item>
								<ContextMenu.Item
									disabled={account.status === 'missing'}
									onSelect={() => openFolder(account)}
								>
									<IconFolderOpen />
									{m.menu_open_folder()}
								</ContextMenu.Item>
								<ContextMenu.Item
									disabled={account.status === 'missing'}
									onSelect={() => createShortcuts(account)}
								>
									<IconAppWindow />
									{m.menu_add_shortcuts()}
								</ContextMenu.Item>
								<ContextMenu.Item onSelect={() => (editingOptions = account)}>
									<IconTerminal2 />
									{m.menu_launch_options()}
								</ContextMenu.Item>
								<ContextMenu.Item onSelect={() => (colouring = account)}>
									<IconPalette />
									{m.menu_badge_colour()}
								</ContextMenu.Item>
								<ContextMenu.Separator />
								<ContextMenu.Item variant="destructive" onSelect={() => askDelete(account)}>
									<IconTrash />
									{m.menu_delete()}
								</ContextMenu.Item>
							</ContextMenu.Content>
</ContextMenu.Root>
			{/each}
		</Item.Group>
	{:else if !loaded}
			<p class="py-10 text-center text-sm text-muted-foreground">{m.loading()}</p>
		{:else if loaded && list.claudeFound}
		<div class="flex flex-col items-center gap-1 py-10 text-center">
			<p class="text-sm font-medium">{m.accounts_empty()}</p>
			<p class="text-xs text-muted-foreground">{m.accounts_empty_hint()}</p>
		</div>
	{/if}
</section>

{#snippet usageWindow(label: string, window: WindowUsage | null)}
	{#if window}
		<div class="flex flex-col gap-1.5 text-xs">
			<div class="flex items-baseline justify-between">
				<span class="text-muted-foreground">{label}</span>
				<span class="tabular-nums">{formatPercentValue(Math.round(window.utilization), formatTag())}</span>
			</div>
			<Progress
				value={window.utilization}
				aria-label={label}
				class={window.utilization >= 100 ? '*:data-[slot=progress-indicator]:bg-destructive' : ''}
			/>
			{#if window.resetsAt}
				<p class="text-[11px] text-muted-foreground">
					{m.usage_resets({ time: formatReset(window.resetsAt, new Date(now), formatTag()) })}
				</p>
			{/if}
		</div>
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

<Dialog.Root
	open={colouring !== null}
	onOpenChange={(open) => {
		if (!open) colouring = null;
	}}
>
	<Dialog.Content class="sm:max-w-sm">
		<Dialog.Header>
			<Dialog.Title>{m.menu_badge_colour()}</Dialog.Title>
		</Dialog.Header>
		<div class="grid grid-cols-5 justify-items-center gap-3">
			{#each list.palette as colour (colour)}
				<button
					type="button"
					aria-label={colour}
					class="flex size-9 items-center justify-center rounded-full text-white outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
					style="background-color: {colour}"
					onclick={() => colouring && changeColour(colouring, colour)}
				>
					{#if colour === colouring?.colour}
						<IconCheck />
					{/if}
				</button>
			{/each}
		</div>
	</Dialog.Content>
</Dialog.Root>

{#if editingOptions}
	<LaunchOptionsDialog
		account={editingOptions}
		onclose={() => (editingOptions = null)}
		onsaved={() => {
			editingOptions = null;
			void refresh();
		}}
	/>
{/if}

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
