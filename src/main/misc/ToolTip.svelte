<script>
    import { afterUpdate } from "svelte";
    export let text;
    
    let element;
    let shown = false;
    afterUpdate(() => {
        element.parentNode.addEventListener("mouseenter", e => {
            shown = true;
        });
        element.parentNode.addEventListener("mouseleave", e => {
            shown = false;
        });
    });
</script>

<div bind:this={element}>
    {#if shown}
        <div class="tooltip">{text}</div>
    {/if}
</div>

<style>
    .tooltip {
        background-color: black;
        color: white;
        padding: 7px 10px;
        border-radius: 6px;
        font-size: 12px;
        font-weight: bold;
        position: absolute;
        white-space: nowrap;
        left: 50%;
        top: 0;
        transform: translate(-50%, -42px);
        z-index: 1000;
    }

    .tooltip::after {
        content: "";
        display: block;
        height: 9px;
        width: 9px;
        background-color: black;
        position: absolute;
        left: 50%;
        transform: translate(-50%, 2px) rotate(45deg);
    }
</style>