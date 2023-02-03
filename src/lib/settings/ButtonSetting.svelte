<script>
    import { createEventDispatcher } from "svelte";

    export let text;
    export let color;

    const dispatch = createEventDispatcher();
</script>

<button class="button" type="button" on:click={e => dispatch("click", e)} style="background-color: {color};">{text}</button>

<style>
    .button {
        font-family: "Roboto", sans-serif;
        color: white;
        font-size: 14px;
        border-radius: 6px;
        padding: 7px;
        transition: ease filter .2s;
        cursor: pointer;
        border: none;
        font-weight: 600;
    }

    .button:hover {
        filter: brightness(90%);
    }
</style>