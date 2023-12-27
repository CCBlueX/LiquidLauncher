<script>
    import { createEventDispatcher } from "svelte";
    import ToggleSetting from "./ToggleSetting.svelte";

    export let value;
    export let title;

    const dispatch = createEventDispatcher();
</script>

<div class="custom-mod-setting">
    <ToggleSetting bind:value={value} {title} disabled={false} on:change />
    <button class="button-delete" on:click={() => dispatch("delete", { name: title })}>
        <img src="img/icon/icon-button-close.svg" alt="delete" title="Remove mod">
    </button>
</div>

<style>
    .custom-mod-setting {
        display: grid;
        grid-template-columns: 1fr max-content;
        gap: 5px;
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