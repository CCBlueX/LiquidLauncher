<script>
    import { open } from "@tauri-apps/api/shell";
    import ToolTip from "../ToolTip.svelte";

    export let text;
    export let icon;
    export let url;

    async function handleClick(e) {
        await open(url);
    }
</script>

<button class="button" type="button" on:click={handleClick}>
    <ToolTip {text} />

    <img class="icon" src="img/icon/social/icon-{icon}.svg" alt="icon"> 
</button>

<style>
    .button {
        position: relative;
        background-color: transparent;
        border: none;
        display: flex;
        justify-items: center;
        cursor: pointer;
    }

    .icon {
        height: 22px;
        width: 22px;
    }
</style>