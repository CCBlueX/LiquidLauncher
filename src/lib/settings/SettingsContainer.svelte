<script>
    import { fly } from "svelte/transition";
    import { createEventDispatcher } from "svelte";

    export let title;

    const dispatch = createEventDispatcher();

    function handleHideClick(e) {
        dispatch("hideSettings");
    }
</script>

<div class="container" transition:fly={{ y: -10, duration: 200 }}>
    <div class="header">
        <div class="title">{title}</div>
        <button class="button-hide" on:click={handleHideClick}>
            <img class="icon" src="img/icon/icon-button-close.svg" alt="hide">
        </button>
    </div>

    <div class="settings">
        <slot />
    </div>
</div>

<style>
    .container {
        width: 500px;
        position: fixed;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        background-color: rgba(0, 0, 0, 0.58);
        backdrop-filter: blur(10px);
        padding: 25px;
        border-radius: 6px;
        z-index: 1000;
        max-height: calc(100% - 140px);
        overflow: hidden;
        display: flex;
        flex-direction: column;
        box-shadow: 0 0 10px rgba(0, 0, 0, 0.5);
    }

    .header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 20px;
    }

    .title {
        font-size: 16px;
        color: white;
    }

    .button-hide {
        background-color: transparent;
        border: none;
        cursor: pointer;
    }

    .settings {
        background-color: rgba(0, 0, 0, 0.26);
        padding: 10px;
        border-radius: 6px;
        display: flex;
        flex-direction: column;
        row-gap: 20px;
        flex: 1;
        overflow: auto;
    }
</style>