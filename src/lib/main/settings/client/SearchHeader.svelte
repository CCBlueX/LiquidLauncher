<script>
    import { createEventDispatcher } from "svelte";
    import IconButtonSetting from "../../../settings/IconButtonSetting.svelte";
    import TextSetting from "../../../settings/TextSetting.svelte";

    export let placeholder;
    export let query;

    const dispatch = createEventDispatcher();
</script>

<div class="stack">
    <div class="back">
        <IconButtonSetting text="Back" icon="icon-prev" on:click={() => dispatch("back")} />
    </div>
    <TextSetting {placeholder} bind:value={query} on:keydown={e => e.key === "Escape" && (query = "")} />
    <slot />
</div>

<style>
    .stack {
        display: flex;
        flex-direction: column;
        row-gap: 10px;
    }

    .back {
        display: flex;
    }
</style>
