<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { Attachment } from 'svelte/attachments';
	import { createSortable, type CreateSortableInput } from '@dnd-kit/svelte/sortable';

	/**
	 * One row of a sortable list. It must render inside a DragDropProvider; the row's own
	 * element takes the attachment its `children` snippet is given.
	 */
	let {
		id,
		index,
		sensors,
		children
	}: {
		id: string;
		index: number;
		/** The sensors that start a drag of this row, in place of the provider's. */
		sensors?: CreateSortableInput['sensors'];
		children: Snippet<[Attachment<HTMLElement>]>;
	} = $props();

	const sortable = createSortable({
		get id() {
			return id;
		},
		get index() {
			return index;
		},
		get sensors() {
			return sensors;
		}
	});

	const attach: Attachment<HTMLElement> = (node) => sortable.attach(node);
</script>

{@render children(attach)}
