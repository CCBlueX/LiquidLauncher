<script>
    import { createEventDispatcher } from "svelte";

    export let icon;
    export let title;
    export let subtitle;

    const dispatch = createEventDispatcher();
</script>

<button class="button-version" on:click={e => dispatch("click", e)}>
    <img class="icon" src="img/icon/icon-version-{icon}.png" alt={title}>
    <div class="title">{title}</div>
    <div class="subtitle">{subtitle}</div>
</button>

<style>
    .button-version {
        background-color: rgba(0, 0, 0, .26);
        border-radius: 6px;
        text-align: center;
        border: solid 1px transparent;
        padding: 10px;
        transition: ease border-color .2s;
        cursor: pointer;
    }

    .button-version:hover {
        border-color: #4677FF;
    }

    .button-version .icon {
        margin-bottom: 5px;
    }

    .button-version .title {
        color: white;
        font-size: 12px;
        margin-bottom: 2px;
    }

    .button-version .subtitle {
        font-size: 10px;
        color: rgba(255, 255, 255, .5);
    }
</style>