<script>
    import { createEventDispatcher } from "svelte";

    export let text;
    export let icon;

    const dispatch = createEventDispatcher();
</script>

<button class="button" type="button" on:click={() => dispatch("click")}>
    <img class="icon" src="img/icon/{icon}.svg" alt={icon}>    
    <span>{text}</span>
</button>

<style>
    .button {
        margin: 0;
        padding: 0;
        border: none;
        background-color: transparent;
        color: white;
        font-family: "Inter", sans-serif;
        font-size: 14px;
        display: grid;
        grid-template-columns: max-content 1fr;
        column-gap: 5px;
        cursor: pointer;
        align-items: center;
    }

    .icon {
        height: 10px;
    }
</style>