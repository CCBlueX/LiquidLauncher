<script>
    import { createEventDispatcher } from "svelte";

    export let text;
    export let primary;

    const dispatch = createEventDispatcher();
</script>

<button class="button" class:primary type="button" on:click={e => dispatch("click", e)}>{text}</button>

<style>
    .button {
        background-color: rgba(0, 0, 0, .26);
        border: none;
        font-family: "Inter", sans-serif;
        padding: 20px;
        color: white;
        font-size: 16px;
        font-weight: 600;
        border-radius: 6px;
        cursor: pointer;
        margin: 0 20px;
        transition: ease background-color .2s;
    }

    .button.primary {
        background-color: #4677FF;
    }

    .button.primary:hover {
        background-color: #3E69E2;
    }
</style>