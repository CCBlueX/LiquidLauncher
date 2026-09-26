<script>
    import { createEventDispatcher, onDestroy } from "svelte";
    import SettingWrapper from "../../../settings/SettingWrapper.svelte";
    import IconButtonSetting from "../../../settings/IconButtonSetting.svelte";
    import TextSetting from "../../../settings/TextSetting.svelte";
    import ButtonSetting from "../../../settings/ButtonSetting.svelte";
    import RippleLoader from "../../../common/RippleLoader.svelte";
    import Message from "./Message.svelte";

    export let placeholder;
    export let title;
    /** What the error says when `search` fails. */
    export let failure;
    /** Searches for a query; searching again after typing waits for a pause. */
    export let search;
    export let query = "";
    export let result = null;

    const dispatch = createEventDispatcher();

    let error = null;
    let request = 0;
    let timer;

    /** Searches again. `quiet` keeps the result shown until the new one is in. */
    export async function load(quiet = false) {
        const current = ++request;
        if (!quiet) {
            result = null;
            error = null;
        }

        try {
            const found = await search(query);
            if (current === request) {
                result = found;
                error = null;
            }
        } catch (e) {
            console.error(`${failure}`, e);
            if (current === request) error = `${e}`;
        }
    }

    let searched = query;
    $: if (query !== searched) {
        searched = query;
        clearTimeout(timer);
        timer = setTimeout(load, 300);
    }

    onDestroy(() => clearTimeout(timer));
    load();
</script>

<div class="stack">
    <div class="back">
        <IconButtonSetting text="Back" icon="icon-prev" on:click={() => dispatch("back")} />
    </div>
    <TextSetting {placeholder} bind:value={query} on:keydown={e => e.key === "Escape" && (query = "")} />
    <slot name="controls" />
</div>

<SettingWrapper {title} unbounded>
    <span slot="title-element" class="aside"><slot name="aside" {result} /></span>
    {#if error}
        <Message title={failure} note={error}>
            <ButtonSetting small text="Try again" on:click={() => load()} />
        </Message>
    {:else if !result}
        <Message>
            <RippleLoader size={80} />
        </Message>
    {:else}
        <slot {result} />
    {/if}
</SettingWrapper>

<style>
    .stack {
        display: flex;
        flex-direction: column;
        row-gap: 10px;
    }

    .back {
        display: flex;
    }

    .aside {
        font-size: 12px;
        line-height: 17px;
        color: rgba(255, 255, 255, .5);
    }
</style>
