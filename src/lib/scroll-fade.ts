/** Distance over which the top and bottom edges fade while there is more to scroll. */
const FADE = '24px';

/**
 * Fades the top and bottom edges of a scroll container only while content is hidden beyond them.
 * The `.scroll-fade` rule in `layout.css` reads the two custom properties this sets.
 */
export function scrollFade(node: HTMLElement) {
	const update = () => {
		const max = node.scrollHeight - node.clientHeight;
		node.style.setProperty('--fade-top', node.scrollTop > 1 ? FADE : '0px');
		node.style.setProperty('--fade-bottom', node.scrollTop < max - 1 ? FADE : '0px');
	};

	const resizes = new ResizeObserver(update);
	resizes.observe(node);
	if (node.firstElementChild) resizes.observe(node.firstElementChild);
	node.addEventListener('scroll', update, { passive: true });
	node.classList.add('scroll-fade');
	update();

	return () => {
		node.removeEventListener('scroll', update);
		resizes.disconnect();
		node.classList.remove('scroll-fade');
		node.style.removeProperty('--fade-top');
		node.style.removeProperty('--fade-bottom');
	};
}
