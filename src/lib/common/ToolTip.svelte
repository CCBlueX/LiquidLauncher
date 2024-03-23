<script>
    import { fly } from "svelte/transition";
    import { afterUpdate } from "svelte";

    export let text;

    let element;
    let shown = false;

    afterUpdate(() => {
        element.parentNode.addEventListener("mouseenter", (e) => {
            shown = true;
        });

        element.parentNode.addEventListener("mouseleave", (e) => {
            shown = false;
        });
    });
</script>

<div bind:this={element}>
    {#if shown}
        <div transition:fly={{ y: -10, duration: 200 }} class="tooltip">
            {text}
        </div>
    {/if}
</div>

<style>
    .tooltip {
        background-color: #4677FF;
        color: white;
        padding: 7px 10px;
        border-radius: 15px;
        font-size: 14px;
        font-weight: 600;
        position: absolute;
        white-space: nowrap;
        left: 50%;
        top: 0;
        transform: translate(-50%, -45px);
        z-index: 1000;
    }

    .tooltip::after {
        content: "";
        display: block;
        height: 10px;
        width: 10px;
        background-color: #4677FF;
        position: absolute;
        left: 50%;
        transform: translateX(-50%) rotate(45deg);
    }
</style>
