<script>
    import { createEventDispatcher } from "svelte";
    import ToggleSetting from "./ToggleSetting.svelte";

    export let value;
    export let title;
    export let lined = false;

    const dispatch = createEventDispatcher();
</script>

<div class="custom-mod-setting">
    <div>
        <ToggleSetting bind:value={value} {title} disabled={false} on:change />
        {#if lined}
            <div class="line"><slot name="line" /></div>
        {/if}
    </div>
    <div class="side">
        <button class="button-delete" on:click={() => dispatch("delete", { name: title })}>
            <img src="img/icon/icon-button-close.svg" alt="delete" title="Remove mod">
        </button>
        <slot name="side" />
    </div>
</div>

<style>
    .custom-mod-setting {
        display: grid;
        grid-template-columns: minmax(0, 1fr) max-content;
        gap: 5px;
        align-items: center;
    }

    .line {
        padding-left: 30px;
        font-size: 12px;
        line-height: 15px;
        color: rgba(255, 255, 255, .5);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .side {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .button-delete {
        background-color: transparent;
        display: flex;
        align-items: center;
        margin: 0;
        padding: 0;
        border: none;
        cursor: pointer;
        opacity: 0;
        transition: ease opacity .2s;
    }

    .custom-mod-setting:hover .button-delete {
        opacity: 1;
    }

    .button-delete img {
        height: 10px;
    }
</style>
