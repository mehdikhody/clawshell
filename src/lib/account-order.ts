/**
 * The account ids with `dragged` moved to just before or after `target`.
 * Null when nothing would change: the same row, an unknown id, or a drop that
 * leaves the order as it is.
 */
export function moveAccount(
	ids: string[],
	dragged: string,
	target: string,
	position: 'before' | 'after'
): string[] | null {
	if (dragged === target || !ids.includes(dragged) || !ids.includes(target)) return null;
	const rest = ids.filter((id) => id !== dragged);
	const at = rest.indexOf(target) + (position === 'after' ? 1 : 0);
	const moved = [...rest.slice(0, at), dragged, ...rest.slice(at)];
	return moved.every((id, i) => id === ids[i]) ? null : moved;
}
