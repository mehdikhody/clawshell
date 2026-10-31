<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import IconAlertTriangle from '@tabler/icons-svelte/icons/alert-triangle';
	import { errorCode, type Account } from '$lib/accounts';
	import * as Alert from '$lib/components/ui/alert';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { formatTag } from '$lib/locale-state.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { folderName, listSessions, type CodeSession } from '$lib/sessions';
	import { formatAgo } from '$lib/usage';

	let {
		account,
		onclose
	}: {
		account: Account;
		onclose: () => void;
	} = $props();

	/** Null until the core has answered. */
	let sessions = $state.raw<CodeSession[] | null>(null);
	let problem = $state<string | null>(null);
	/** The `cliSessionId`s the user has ticked. */
	const selected = new SvelteSet<string>();
	// The dialog is mounted per opening, so "ago" is measured from then.
	const now = Date.now();

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
</script>

<Dialog.Root
	open
	onOpenChange={(open) => {
		if (!open) onclose();
	}}
>
	<Dialog.Content class="sm:max-w-lg">
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
					<Skeleton class="h-10 w-full" />
				{/each}
			</div>
		{:else if sessions.length === 0}
			<p class="py-6 text-center text-sm text-muted-foreground">{m.copy_sessions_empty()}</p>
		{:else}
			<ul class="flex max-h-80 flex-col gap-1 overflow-y-auto">
				{#each sessions as session (session.cliSessionId)}
					<li>
						<label
							class="flex cursor-pointer items-start gap-3 rounded-md px-2 py-1.5 hover:bg-accent has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring"
						>
							<input
								type="checkbox"
								class="mt-1 size-4 shrink-0 accent-primary"
								checked={selected.has(session.cliSessionId)}
								onchange={(event) => toggle(session.cliSessionId, event.currentTarget.checked)}
							/>
							<span class="flex min-w-0 flex-1 flex-col gap-0.5">
								<span class="flex items-center gap-2">
									<span class="truncate text-sm font-medium">
										{session.title || m.copy_sessions_untitled()}
									</span>
									{#if session.isArchived}
										<Badge variant="outline">{m.copy_sessions_archived()}</Badge>
									{/if}
								</span>
								<span class="flex items-center gap-2 text-xs text-muted-foreground">
									<span class="truncate" dir="ltr">
										{folderName(session.cwd) ?? m.copy_sessions_no_folder()}
									</span>
									<span class="shrink-0">·</span>
									<span class="shrink-0">{formatAgo(session.lastActivityAt, now, formatTag())}</span
									>
								</span>
							</span>
						</label>
					</li>
				{/each}
			</ul>
		{/if}

		<Dialog.Footer class="items-center">
			{#if selected.size > 0}
				<span class="text-xs text-muted-foreground sm:me-auto">
					{m.copy_sessions_selected({ count: selected.size })}
				</span>
			{/if}
			<Dialog.Close type="button">
				{#snippet child({ props })}
					<Button {...props} variant="outline">{m.action_cancel()}</Button>
				{/snippet}
			</Dialog.Close>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
