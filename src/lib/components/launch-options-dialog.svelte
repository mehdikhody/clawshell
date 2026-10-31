<script lang="ts">
	import IconAlertTriangle from '@tabler/icons-svelte/icons/alert-triangle';
	import IconInfoCircle from '@tabler/icons-svelte/icons/info-circle';
	import IconPlus from '@tabler/icons-svelte/icons/plus';
	import IconX from '@tabler/icons-svelte/icons/x';
	import { errorCode, setLaunchOptions, type Account } from '$lib/accounts';
	import * as Alert from '$lib/components/ui/alert';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Input } from '$lib/components/ui/input';
	import { m } from '$lib/paraglide/messages.js';

	let {
		account,
		onclose,
		onsaved
	}: {
		account: Account;
		onclose: () => void;
		onsaved: () => void;
	} = $props();

	/** Rows carry a key so removing one leaves the others' typing alone. */
	type EnvRow = { key: number; name: string; value: string };
	type FlagRow = { key: number; value: string };

	let nextKey = 0;
	// The dialog is mounted per edit, so it starts from the account as it is now.
	const startEnv = (): EnvRow[] =>
		account.env.map((v) => ({ key: nextKey++, name: v.name, value: v.value }));
	const startFlags = (): FlagRow[] => account.flags.map((value) => ({ key: nextKey++, value }));
	let env = $state(startEnv());
	let flags = $state(startFlags());
	let saving = $state(false);
	let problem = $state<string | null>(null);

	const reasons: Record<string, () => string> = {
		'invalid-env-name': () => m.error_env_name_invalid(),
		'duplicate-env-name': () => m.error_env_name_duplicate(),
		'invalid-env-value': () => m.error_env_value_invalid(),
		'reserved-flag': () => m.error_flag_reserved()
	};

	async function save() {
		saving = true;
		problem = null;
		try {
			await setLaunchOptions(account.id, {
				// A row left completely blank is just an unused row.
				env: env
					.filter((row) => row.name !== '' || row.value !== '')
					.map(({ name, value }) => ({ name, value })),
				flags: flags.map((row) => row.value).filter((value) => value.trim() !== '')
			});
			onsaved();
		} catch (error) {
			problem = reasons[errorCode(error) ?? '']?.() ?? m.error_launch_options_failed();
		} finally {
			saving = false;
		}
	}
</script>

<Dialog.Root
	open
	onOpenChange={(open) => {
		if (!open) onclose();
	}}
>
	<Dialog.Content class="max-h-[85vh] overflow-y-auto sm:max-w-lg">
		<form
			class="flex flex-col gap-4"
			onsubmit={(event) => {
				event.preventDefault();
				void save();
			}}
		>
			<Dialog.Header>
				<Dialog.Title>{m.launch_options_title()}</Dialog.Title>
				<Dialog.Description class="flex items-start gap-1.5">
					<IconInfoCircle class="mt-0.5 size-4 shrink-0" />
					<span>{m.launch_options_note()}</span>
				</Dialog.Description>
			</Dialog.Header>

			{#if problem}
				<Alert.Root variant="destructive">
					<IconAlertTriangle />
					<Alert.Title>{problem}</Alert.Title>
				</Alert.Root>
			{/if}

			<fieldset class="flex flex-col gap-2">
				<legend class="text-sm font-medium">{m.launch_options_env()}</legend>
				<p class="text-xs text-muted-foreground">{m.launch_options_env_hint()}</p>
				{#each env as row, i (row.key)}
					<div class="grid grid-cols-[1fr_1fr_auto] items-center gap-2">
						<Input
							aria-label={m.launch_options_env_name()}
							placeholder="NAME"
							class="font-mono"
							dir="ltr"
							autocomplete="off"
							spellcheck={false}
							bind:value={row.name}
						/>
						<Input
							aria-label={m.launch_options_env_value()}
							class="font-mono"
							dir="ltr"
							autocomplete="off"
							spellcheck={false}
							bind:value={row.value}
						/>
						<Button
							type="button"
							variant="ghost"
							size="icon-sm"
							aria-label={m.launch_options_env_remove({ index: i + 1 })}
							onclick={() => (env = env.filter((r) => r.key !== row.key))}
						>
							<IconX />
						</Button>
					</div>
				{/each}
				<Button
					type="button"
					variant="outline"
					size="sm"
					class="self-start"
					onclick={() => (env = [...env, { key: nextKey++, name: '', value: '' }])}
				>
					<IconPlus />
					{m.launch_options_env_add()}
				</Button>
			</fieldset>

			<fieldset class="flex flex-col gap-2">
				<legend class="text-sm font-medium">{m.launch_options_flags()}</legend>
				<p class="text-xs text-muted-foreground">{m.launch_options_flags_hint()}</p>
				{#each flags as row, i (row.key)}
					<div class="grid grid-cols-[1fr_auto] items-center gap-2">
						<Input
							aria-label={m.launch_options_flag()}
							placeholder="--flag=value"
							class="font-mono"
							dir="ltr"
							autocomplete="off"
							spellcheck={false}
							bind:value={row.value}
						/>
						<Button
							type="button"
							variant="ghost"
							size="icon-sm"
							aria-label={m.launch_options_flag_remove({ index: i + 1 })}
							onclick={() => (flags = flags.filter((r) => r.key !== row.key))}
						>
							<IconX />
						</Button>
					</div>
				{/each}
				<Button
					type="button"
					variant="outline"
					size="sm"
					class="self-start"
					onclick={() => (flags = [...flags, { key: nextKey++, value: '' }])}
				>
					<IconPlus />
					{m.launch_options_flag_add()}
				</Button>
			</fieldset>

			<Dialog.Footer>
				<Dialog.Close type="button">
					{#snippet child({ props })}
						<Button {...props} variant="outline">{m.action_cancel()}</Button>
					{/snippet}
				</Dialog.Close>
				<Button type="submit" disabled={saving}>{m.launch_options_save()}</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
