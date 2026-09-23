<script>
    import { createEventDispatcher } from "svelte";

    export let name;
    export let description = null;
    export let removable = false;
    export let dim = false;
    export let openable = true;

    const dispatch = createEventDispatcher();
</script>

<div class="row" class:dim class:openable>
    <button class="open" type="button" disabled={!openable} on:click={() => dispatch("open")}>
        <span class="name">{name}</span>
        {#if description}
            <span class="line">{description}</span>
        {/if}
        <span class="line"><slot /></span>
    </button>
    <div class="side">
        {#if removable}
            <button class="remove" type="button" title="Remove" aria-label="Remove {name}"
                    on:click={() => dispatch("remove")}>
                <img src="img/icon/icon-button-close.svg" alt="">
            </button>
        {/if}
        <slot name="side" />
    </div>
    {#if openable}
        <img class="chevron" src="img/icon/icon-next.svg" alt="" aria-hidden="true">
    {/if}
</div>

<style>
    .row {
        position: relative;
        display: grid;
        grid-template-columns: minmax(0, 1fr) max-content max-content;
        column-gap: 10px;
        align-items: center;
        margin: 0 -6px;
        padding: 3px 5px;
        border: solid 1px transparent;
        border-radius: 6px;
        transition: ease border-color .2s;
    }

    .row:not(.openable) {
        grid-template-columns: minmax(0, 1fr) max-content;
    }

    .row.openable:hover, .row.openable:focus-within {
        border-color: #4677FF;
    }

    .open {
        display: grid;
        text-align: left;
        background: transparent;
        border: none;
        color: white;
        font-family: "Inter", sans-serif;
        font-size: 14px;
        cursor: pointer;
        min-width: 0;
    }

    .open:disabled {
        cursor: default;
    }

    .open::after {
        content: "";
        position: absolute;
        inset: 0;
    }

    .open > span {
        display: block;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .name {
        line-height: 17px;
    }

    .dim .name {
        color: rgba(255, 255, 255, .5);
    }

    .line {
        font-size: 12px;
        line-height: 15px;
        color: rgba(255, 255, 255, .5);
    }

    .side {
        display: flex;
        align-items: center;
        gap: 10px;
        position: relative;
        z-index: 1;
    }

    .remove {
        background: transparent;
        border: none;
        display: flex;
        align-items: center;
        cursor: pointer;
        opacity: 0;
        transition: ease opacity .2s;
        padding: 0;
    }

    .remove img {
        height: 10px;
    }

    .row:hover .remove, .row:focus-within .remove {
        opacity: 1;
    }

    .chevron {
        height: 10px;
        opacity: .5;
    }
</style>
