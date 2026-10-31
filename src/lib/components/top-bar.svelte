<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import IconMinus from '@tabler/icons-svelte/icons/minus';
	import IconX from '@tabler/icons-svelte/icons/x';
	import { Button } from '$lib/components/ui/button';
	import { m } from '$lib/paraglide/messages.js';
</script>

<header
	data-tauri-drag-region
	class="flex h-11 shrink-0 items-center justify-between border-b ps-4 pe-1.5 select-none"
>
	<span data-tauri-drag-region class="text-sm font-semibold tracking-tight">{m.app_name()}</span>
	<div class="flex items-center gap-0.5">
		<Button
			variant="ghost"
			size="icon-sm"
			aria-label={m.window_minimize()}
			onclick={() => getCurrentWindow().minimize()}
		>
			<IconMinus />
		</Button>
		<Button
			variant="ghost"
			size="icon-sm"
			aria-label={m.window_close()}
			class="hover:bg-destructive hover:text-white"
			onclick={() => getCurrentWindow().close()}
		>
			<IconX />
		</Button>
	</div>
</header>
