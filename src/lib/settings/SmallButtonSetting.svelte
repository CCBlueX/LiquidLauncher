<script>
    import { createEventDispatcher } from "svelte";

    export let text;
    export let disabled = false;

    const dispatch = createEventDispatcher();
</script>

<button {disabled} class="button" type="button" on:click={e => dispatch("click", e)}>{text}</button>

<style>
    /* The news "Read more" button. */
    .button {
        border: none;
        background-color: #4677FF;
        color: white;
        font-size: 12px;
        border-radius: 4px;
        height: 30px;
        transition: background-color ease .2s, ease opacity .2s;
        padding: 0 10px;
        font-family: "Inter", sans-serif;
        cursor: pointer;
    }

    .button:hover {
        background-color: #3E69E2;
    }

    .button:disabled {
        pointer-events: none;
        opacity: .6;
    }
</style>
