<script>
    import { createEventDispatcher } from "svelte";
    import IconButtonSetting from "../../../settings/IconButtonSetting.svelte";

    export let title = null;
    export let note = null;
    export let clearable = false;

    const dispatch = createEventDispatcher();
</script>

<div class="center">
    {#if title}
        <div class="strong">{title}</div>
    {/if}
    {#if note}
        <div class="note">{note}</div>
    {/if}
    <slot />
    {#if clearable}
        <div class="clear">
            <IconButtonSetting text="Clear search" icon="icon-button-close" on:click={() => dispatch("clear")} />
        </div>
    {/if}
</div>

<style>
    .center {
        display: flex;
        flex-direction: column;
        align-items: center;
        row-gap: 5px;
        padding: 20px 0;
        text-align: center;
    }

    .strong {
        color: white;
    }

    .note {
        font-size: 12px;
        line-height: 15px;
        color: rgba(255, 255, 255, .5);
        word-break: break-word;
    }

    .clear {
        display: flex;
        margin-top: 5px;
    }
</style>
