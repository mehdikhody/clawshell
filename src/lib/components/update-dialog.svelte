<script lang="ts">
	import IconAlertTriangle from '@tabler/icons-svelte/icons/alert-triangle';
	import * as Alert from '$lib/components/ui/alert';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Progress } from '$lib/components/ui/progress';
	import { formatPercentValue } from '$lib/locale';
	import { formatTag } from '$lib/locale-state.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { installUpdate, updateState } from '$lib/update.svelte';
	import { parseNotes, type Inline } from '$lib/release-notes';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	const current = $derived(updateState.current);
	/** While the installer runs there is nothing to cancel, so the dialog stays until the restart. */
	const busy = $derived(current.status === 'downloading' || current.status === 'installing');
	/** Null while the download size is unknown, which draws an indeterminate bar. */
	const percent = $derived(
		current.status === 'downloading' && current.total
			? Math.min(100, (current.downloaded / current.total) * 100)
			: null
	);
</script>

{#snippet inline(runs: Inline[])}
	{#each runs as run, i (i)}
		{#if run.bold}<strong class="font-semibold">{run.text}</strong>{:else if run.code}<code
				class="rounded bg-muted px-1 font-mono">{run.text}</code
			>{:else}{run.text}{/if}
	{/each}
{/snippet}

{#if current.status !== 'idle'}
	<Dialog.Root bind:open>
		<Dialog.Content
			showCloseButton={!busy}
			escapeKeydownBehavior={busy ? 'ignore' : 'close'}
			interactOutsideBehavior={busy ? 'ignore' : 'close'}
			class="sm:max-w-lg"
		>
			<Dialog.Header>
				<Dialog.Title class="text-base font-semibold">
					{m.update_title({ version: current.update.version })}
				</Dialog.Title>
				{#if current.update.body}
					<Dialog.Description>{m.update_notes()}</Dialog.Description>
				{/if}
			</Dialog.Header>

			{#if current.update.body}
				<div
					data-testid="update-notes"
					class="flex max-h-48 flex-col gap-2 overflow-y-auto rounded-md border bg-muted/40 p-3 text-xs"
				>
					{#each parseNotes(current.update.body) as block, i (i)}
						{#if block.kind === 'heading' && block.level < 3}
							<h3 class="mt-1 text-sm font-semibold first:mt-0">{@render inline(block.text)}</h3>
						{:else if block.kind === 'heading'}
							<h4 class="font-semibold">{@render inline(block.text)}</h4>
						{:else if block.kind === 'list'}
							<ul class="flex list-disc flex-col gap-1 ps-4">
								{#each block.items as item, j (j)}
									<li>{@render inline(item)}</li>
								{/each}
							</ul>
						{:else}
							<p>
								{#each block.lines as line, j (j)}
									{#if j > 0}<br />{/if}{@render inline(line)}
								{/each}
							</p>
						{/if}
					{/each}
				</div>
			{/if}

			{#if current.status === 'downloading'}
				<div class="flex flex-col gap-2">
					<div class="flex items-center justify-between text-xs text-muted-foreground">
						<span>{m.update_downloading()}</span>
						{#if percent !== null}
							<span>{formatPercentValue(percent, formatTag())}</span>
						{/if}
					</div>
					<Progress value={percent} />
				</div>
			{:else if current.status === 'installing'}
				<div class="flex flex-col gap-2">
					<span class="text-xs text-muted-foreground">{m.update_installing()}</span>
					<Progress value={null} />
				</div>
			{:else if current.status === 'failed'}
				<Alert.Root variant="destructive">
					<IconAlertTriangle />
					<Alert.Title>{m.update_failed()}</Alert.Title>
					<Alert.Description>{m.update_failed_detail()}</Alert.Description>
				</Alert.Root>
			{/if}

			{#if !busy}
				<Dialog.Footer>
					<Button variant="outline" onclick={() => (open = false)}>{m.update_later()}</Button>
					<Button onclick={() => void installUpdate()}>
						{current.status === 'failed' ? m.update_retry() : m.update_now()}
					</Button>
				</Dialog.Footer>
			{/if}
		</Dialog.Content>
	</Dialog.Root>
{/if}
