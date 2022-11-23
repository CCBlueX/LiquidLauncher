<script>
    import ToolTip from "../ToolTip.svelte";
    import { invoke } from "@tauri-apps/api/tauri";

    export let text;
    export let icon;
    export let url;

    function handleClick(e) {
        invoke("open_url", { url: url });
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