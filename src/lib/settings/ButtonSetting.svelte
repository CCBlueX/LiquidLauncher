<script>
    import { createEventDispatcher } from "svelte";

    export let text;

    const dispatch = createEventDispatcher();
</script>

<button class="button" type="button" on:click={e => dispatch("click", e)}>{text}</button>

<style>
    .button {
        font-family: "Gilroy", sans-serif;
        background-color: #4677FF;
        color: white;
        font-size: 14px;
        border-radius: 6px;
        padding: 7px;
        transition: ease background-color .2s;
        cursor: pointer;
        border: none;
        font-weight: bold;
    }

    .button:hover {
        background-color: #3E69E2;
    }
</style>