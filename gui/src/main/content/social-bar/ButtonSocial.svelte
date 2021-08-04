<script>
    import Tooltip from "../../misc/Tooltip.svelte";

    export let icon;
    export let text;
</script>

<button class="button">
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