<script>
    import Tooltip from "../../misc/ToolTip.svelte";
    import { invoke } from '@tauri-apps/api/tauri'

    export let icon;
    export let text;
    export let url;
</script>

<button class="button" on:click={invoke("open", { url: url })}>
    <Tooltip {text} />
    <img class="icon" src="img/icon/social/icon-{icon}.svg" alt={icon}>
</button>

<style>
    .button {
        background: unset;
        border: none;
        height: 22px;
        width: 22px;
        position: relative;
    }

    .icon {
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
    }
</style>