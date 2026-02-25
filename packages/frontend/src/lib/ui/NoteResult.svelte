<script lang="ts" module>
	export type NoteResult = {
		id: string
		password?: string
	}
</script>

<script lang="ts">
	import { t } from 'svelte-intl-precompile'
	import { status } from '$lib/stores/status'
	import Button from '$lib/ui/Button.svelte'
	import TextInput from '$lib/ui/TextInput.svelte'
	import Canvas from './Canvas.svelte'

	interface Props {
		result: NoteResult
	}

	let { result }: Props = $props()

	let url = $state(`${window.location.origin}/note/${result.id}`)
	if (result.password) url += `#${result.password}`

	function reset() {
		window.location.reload()
	}

	let mailtoHref = $derived(
		`mailto:support@emerald-group.co.uk?subject=${encodeURIComponent('Shared secure note')}&body=${encodeURIComponent(url)}`
	)
</script>

<TextInput
	type="text"
	readonly
	label={$t('common.share_link')}
	value={url}
	copy
	data-testid="share-link"
/>

<div class="qr">
	<Canvas value={url} />
</div>

<div class="actions">
	<a class="mailto-button" href={mailtoHref}>
		✉ send via email
	</a>
</div>

{#if $status?.theme_new_note_notice}
	<p>
		{@html $t('home.new_note_notice')}
	</p>
{/if}
<br />
<Button onclick={reset}>{$t('home.new_note')}</Button>

<style>
	.qr {
		width: min(12rem, 100%);
		margin-top: 1rem;
		margin-bottom: 1rem;
	}

	.actions {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.mailto-button {
		display: inline-block;
		padding: 0.5rem 1rem;
		border: 2px solid var(--ui-clr-primary);
		color: var(--ui-clr-primary);
		text-decoration: none;
		font-size: 0.9rem;
		cursor: pointer;
		transition: background-color 0.15s, color 0.15s;
	}

	.mailto-button:hover {
		background-color: var(--ui-clr-primary);
		color: var(--ui-bg-0, #fff);
	}
</style>
