<script>
    import { createEventDispatcher } from "svelte";


    export let text;
    export let active;

    const dispatch = createEventDispatcher();
</script>

<button class="button" class:active type="button" on:click={e => dispatch("click", e)}>{text}</button>

<style>
    .button {
        background: unset;
        border: none;
        background-color: #4677FF;
        color: white;
        font-size: 14px;
        border-radius: 6px;
        display: block;
        padding: 16px;
        transition: background-color ease .2s;
        font-family: "Inter", sans-serif;
        cursor: pointer;
        width: 100%;
        font-weight: 600;
    }

    .button:hover {
        background-color: #3E69E2;
    }

    .button.active {
        background-color: #B83529;
    }

    .button.active:hover {
        background-color: #9B2D23;
    }
</style>