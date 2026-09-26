<script>
    import { createEventDispatcher } from "svelte";

    export let text;
    export let color = "#4677FF";
    export let disabled = false;
    /** Sized like the news "Read more" button, to sit inside a row. */
    export let small = false;

    const dispatch = createEventDispatcher();
</script>

<button {disabled} class="button" class:small type="button" on:click={e => dispatch("click", e)} style="background-color: {color};">{text}</button>

<style>
    .button {
        font-family: "Inter", sans-serif;
        color: white;
        font-size: 14px;
        border-radius: 6px;
        padding: 7px;
        transition: ease filter .2s, ease opacity .2s;
        cursor: pointer;
        border: none;
        font-weight: 600;
    }

    .button.small {
        font-size: 12px;
        font-weight: normal;
        border-radius: 4px;
        height: 30px;
        padding: 0 10px;
    }

    .button:hover {
        filter: brightness(90%);
    }

    .button:disabled {
        pointer-events: none;
        opacity: .6;
    }
</style>
