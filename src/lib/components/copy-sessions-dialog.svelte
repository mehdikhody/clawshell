<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import IconAlertTriangle from '@tabler/icons-svelte/icons/alert-triangle';
	import IconCheck from '@tabler/icons-svelte/icons/check';
	import IconChevronDown from '@tabler/icons-svelte/icons/chevron-down';
	import IconSelector from '@tabler/icons-svelte/icons/selector';
	import { errorCode, type Account, type CopyBlocker } from '$lib/accounts';
	import * as Alert from '$lib/components/ui/alert';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import * as Command from '$lib/components/ui/command';
	import * as Dialog from '$lib/components/ui/dialog';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import * as Popover from '$lib/components/ui/popover';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { formatTag } from '$lib/locale-state.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { copySessions, folderName, listSessions, type CodeSession } from '$lib/sessions';
	import { formatAgo } from '$lib/usage';

	let {
		account,
		accounts,
		oncopied,
		onclose
	}: {
		account: Account;
		/** Every account; the others are offered as targets. */
		accounts: Account[];
		/** Called with the result line once the copy is done. */
		oncopied: (message: string) => void;
		onclose: () => void;
	} = $props();

	/** Null until the core has answered. */
	let sessions = $state.raw<CodeSession[] | null>(null);
	let problem = $state<string | null>(null);
	/** The `cliSessionId`s the user has ticked. */
	const selected = new SvelteSet<string>();
	// The dialog is mounted per opening, so "ago" is measured from then.
	const now = Date.now();
	let content = $state<HTMLElement | null>(null);
	/** More sessions than this and the picker gets a search box. */
	const SEARCH_FROM = 5;
	let pickingSessions = $state(false);
	let targetId = $state<string | null>(null);
	let copying = $state(false);
	let copyFailure = $state<string | null>(null);

	const allTicked = $derived(!!sessions?.length && selected.size === sessions.length);
	/** The ticked session when there is just one, so the picker can name it. */
	const onlyTicked = $derived(
		selected.size === 1 ? sessions?.find((s) => selected.has(s.cliSessionId)) : undefined
	);
	const targets = $derived(accounts.filter((a) => a.id !== account.id));
	const target = $derived(targets.find((a) => a.id === targetId && !a.copyBlocker) ?? null);
	const canCopy = $derived(selected.size > 0 && target !== null && !copying);

	const blockerText: Record<CopyBlocker, () => string> = {
		'sign-in': () => m.copy_sessions_blocked_sign_in(),
		'open-code-tab': () => m.copy_sessions_blocked_code_tab()
	};

	onMount(async () => {
		try {
			sessions = await listSessions(account.id);
		} catch (error) {
			problem =
				errorCode(error) === 'sessions-layout-changed'
					? m.error_sessions_layout()
					: m.error_sessions_failed();
		}
	});

	function toggle(id: string, ticked: boolean) {
		if (ticked) selected.add(id);
		else selected.delete(id);
	}

	function toggleAll(ticked: boolean) {
		selected.clear();
		if (ticked) for (const session of sessions ?? []) selected.add(session.cliSessionId);
	}

	/** Copies the ticked sessions; the dialog stays open with the reason when it fails. */
	async function copy() {
		if (!canCopy || !target) return;
		copying = true;
		copyFailure = null;
		try {
			const outcome = await copySessions(account.id, [...selected], target.id);
			oncopied(
				outcome.targetRunning
					? m.copy_sessions_copied_restart({ count: outcome.copied, label: target.label })
					: m.copy_sessions_copied({ count: outcome.copied })
			);
		} catch (error) {
			const code = errorCode(error);
			copyFailure =
				code === 'copy-sign-in'
					? m.copy_sessions_blocked_sign_in()
					: code === 'copy-open-code-tab'
						? m.copy_sessions_blocked_code_tab()
						: code === 'sessions-layout-changed'
							? m.error_sessions_layout()
							: m.error_copy_failed();
		} finally {
			copying = false;
		}
	}
</script>

<Dialog.Root
	open
	onOpenChange={(open) => {
		if (!open) onclose();
	}}
>
	<!-- Focus the dialog itself, so no button wears a focus ring before the user tabs. -->
	<Dialog.Content
		bind:ref={content}
		class="sm:max-w-md"
		onOpenAutoFocus={(event) => {
			event.preventDefault();
			content?.focus();
		}}
	>
		<Dialog.Header>
			<Dialog.Title class="text-base font-semibold">
				{m.copy_sessions_title({ label: account.label })}
			</Dialog.Title>
		</Dialog.Header>

		{#if problem}
			<Alert.Root variant="destructive">
				<IconAlertTriangle />
				<Alert.Title>{problem}</Alert.Title>
			</Alert.Root>
		{:else if sessions === null}
			<div class="flex flex-col gap-2" aria-busy="true">
				{#each [0, 1, 2] as n (n)}
					<Skeleton class="h-12 w-full" />
				{/each}
			</div>
		{:else if sessions.length === 0}
			<p class="py-6 text-center text-sm text-muted-foreground">{m.copy_sessions_empty()}</p>
		{:else}
			<div class="flex flex-col gap-4">
				<div class="flex min-w-0 flex-col gap-2">
					<span id="copy-sessions-label" class="text-sm font-medium">
						{m.copy_sessions_sessions()}
					</span>
					<Popover.Root bind:open={pickingSessions}>
						<Popover.Trigger>
							{#snippet child({ props })}
								<Button
									{...props}
									id="copy-sessions"
									variant="outline"
									role="combobox"
									aria-expanded={pickingSessions}
									class="min-w-0 justify-between"
									aria-labelledby="copy-sessions-label copy-sessions"
								>
									{#if selected.size === 0}
										<span class="text-muted-foreground">{m.copy_sessions_choose()}</span>
									{:else if selected.size === 1 && onlyTicked}
										<span class="truncate">{onlyTicked.title || m.copy_sessions_untitled()}</span>
									{:else}
										<span class="truncate"
											>{m.copy_sessions_selected({ count: selected.size })}</span
										>
									{/if}
									<IconSelector class="opacity-50" />
								</Button>
							{/snippet}
						</Popover.Trigger>
						<Popover.Content
							align="start"
							class="w-(--bits-popover-anchor-width) min-w-72 gap-0 p-0"
						>
							<Command.Root>
								{#if sessions.length > SEARCH_FROM}
									<Command.Input placeholder={m.copy_sessions_search()} />
								{/if}
								<label
									class="mx-1 mt-1 flex cursor-pointer items-center gap-2.5 rounded-sm px-2 py-1.5 text-xs font-medium hover:bg-muted"
								>
									<Checkbox
										checked={allTicked}
										indeterminate={selected.size > 0 && !allTicked}
										onCheckedChange={toggleAll}
									/>
									<span class="flex-1">{m.copy_sessions_select_all()}</span>
									{#if selected.size > 0}
										<span class="text-muted-foreground tabular-nums">
											{m.copy_sessions_selected({ count: selected.size })}
										</span>
									{/if}
								</label>
								<Command.Separator class="my-1" />
								<Command.List>
									<Command.Empty>{m.copy_sessions_no_match()}</Command.Empty>
									{#each sessions as session (session.cliSessionId)}
										{@const ticked = selected.has(session.cliSessionId)}
										{@const folder = folderName(session.cwd)}
										<Command.Item
											value={session.cliSessionId}
											keywords={[session.title, folder ?? '']}
											aria-checked={ticked}
											class={ticked ? 'bg-primary/10' : ''}
											onSelect={() => toggle(session.cliSessionId, !ticked)}
										>
											<span
												class={[
													'flex size-4 shrink-0 items-center justify-center rounded-[4px] border',
													ticked
														? 'border-primary bg-primary text-primary-foreground'
														: 'border-input'
												]}
												aria-hidden="true"
											>
												{#if ticked}<IconCheck class="size-3.5" />{/if}
											</span>
											<span class="flex min-w-0 flex-1 flex-col gap-0.5">
												<span class="flex items-center gap-2">
													<span class="truncate font-medium">
														{session.title || m.copy_sessions_untitled()}
													</span>
													{#if session.isArchived}
														<Badge variant="outline">{m.copy_sessions_archived()}</Badge>
													{/if}
												</span>
												<span class="flex items-center gap-1.5 text-xs text-muted-foreground">
													<span class="truncate" dir="ltr">
														{folder ?? m.copy_sessions_no_folder()}
													</span>
													<span class="shrink-0" aria-hidden="true">·</span>
													<span class="shrink-0"
														>{formatAgo(session.lastActivityAt, now, formatTag())}</span
													>
												</span>
											</span>
										</Command.Item>
									{/each}
								</Command.List>
							</Command.Root>
						</Popover.Content>
					</Popover.Root>
				</div>

				<div class="flex min-w-0 flex-col gap-2">
					<span id="copy-target-label" class="text-sm font-medium">
						{m.copy_sessions_target()}
					</span>
					<DropdownMenu.Root>
						<DropdownMenu.Trigger>
							{#snippet child({ props })}
								<Button
									{...props}
									id="copy-target"
									variant="outline"
									class="min-w-0 justify-between"
									aria-labelledby="copy-target-label copy-target"
								>
									{#if target}
										<span class="flex min-w-0 items-center gap-2">
											<span
												class="size-2.5 shrink-0 rounded-full"
												style:background-color={target.colour}
												aria-hidden="true"
											></span>
											<span class="truncate">{target.label}</span>
										</span>
									{:else}
										<span class="text-muted-foreground">{m.copy_sessions_choose_target()}</span>
									{/if}
									<IconChevronDown />
								</Button>
							{/snippet}
						</DropdownMenu.Trigger>
						<DropdownMenu.Content align="end" class="w-(--bits-dropdown-menu-anchor-width)">
							<DropdownMenu.RadioGroup
								value={targetId ?? ''}
								onValueChange={(next) => {
									targetId = next;
									copyFailure = null;
								}}
							>
								{#each targets as option (option.id)}
									<DropdownMenu.RadioItem
										value={option.id}
										disabled={!!option.copyBlocker}
										closeOnSelect
									>
										<span
											class="size-2.5 shrink-0 rounded-full"
											style:background-color={option.colour}
											aria-hidden="true"
										></span>
										<span class="flex min-w-0 flex-col">
											<span class="truncate">{option.label}</span>
											{#if option.copyBlocker}
												<span class="text-xs text-muted-foreground">
													{blockerText[option.copyBlocker]()}
												</span>
											{/if}
										</span>
									</DropdownMenu.RadioItem>
								{/each}
							</DropdownMenu.RadioGroup>
						</DropdownMenu.Content>
					</DropdownMenu.Root>
				</div>
			</div>

			{#if copyFailure}
				<Alert.Root variant="destructive">
					<IconAlertTriangle />
					<Alert.Title>{copyFailure}</Alert.Title>
				</Alert.Root>
			{/if}
		{/if}

		<Dialog.Footer>
			<Dialog.Close type="button">
				{#snippet child({ props })}
					<Button {...props} variant="outline">{m.action_cancel()}</Button>
				{/snippet}
			</Dialog.Close>
			{#if sessions?.length}
				<Button disabled={!canCopy} onclick={copy}>
					{copying ? m.copy_sessions_copying() : m.copy_sessions_copy()}
				</Button>
			{/if}
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
