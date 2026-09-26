<script>
    import { fade } from "svelte/transition";

    /** The images of an item's description, `{ url, caption }`. */
    export let screenshots;

    let failed = new Set();
    let index = 0;
    let viewing = false;

    $: shown = screenshots.filter(screenshot => !failed.has(screenshot.url));
    $: at = Math.min(index, shown.length - 1);
    $: current = shown[at];

    function step(by) {
        index = (at + by + shown.length) % shown.length;
    }

    function keydown(e) {
        if (!viewing) return;
        if (e.key === "Escape") viewing = false;
        if (e.key === "ArrowLeft") step(-1);
        if (e.key === "ArrowRight") step(1);
    }

    /** Moves the viewer out of the settings panel, which would clip it. */
    function toBody(node) {
        document.body.appendChild(node);
        return { destroy: () => node.remove() };
    }
</script>

<svelte:window on:keydown={keydown} />

{#if current}
    <div class="gallery">
        <button class="screenshot" type="button" title="View full size" on:click={() => viewing = true}>
            <img src={current.url} alt={current.caption} on:error={() => failed = new Set(failed).add(current.url)}>
        </button>
        {#if shown.length > 1}
            <button class="step previous" type="button" aria-label="Previous screenshot" on:click={() => step(-1)}>
                <img src="img/icon/icon-prev.svg" alt="">
            </button>
            <button class="step next" type="button" aria-label="Next screenshot" on:click={() => step(1)}>
                <img src="img/icon/icon-next.svg" alt="">
            </button>
        {/if}
    </div>
    <div class="caption">
        <span>{current.caption}</span>
        {#if shown.length > 1}
            <span>{at + 1} / {shown.length}</span>
        {/if}
    </div>

    {#if viewing}
        <button class="viewer" type="button" aria-label="Close" use:toBody transition:fade={{ duration: 150 }}
                on:click={() => viewing = false}>
            <img src={current.url} alt={current.caption}>
            {#if current.caption}
                <span>{current.caption}</span>
            {/if}
        </button>
    {/if}
{/if}

<style>
    .gallery {
        position: relative;
    }

    .screenshot {
        display: block;
        width: 100%;
        aspect-ratio: 16 / 9;
        padding: 0;
        border: none;
        border-radius: 6px;
        overflow: hidden;
        background-color: rgba(0, 0, 0, .26);
        cursor: zoom-in;
    }

    .screenshot img {
        display: block;
        width: 100%;
        height: 100%;
        object-fit: contain;
    }

    .step {
        position: absolute;
        top: 50%;
        transform: translateY(-50%);
        width: 26px;
        height: 26px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: none;
        border-radius: 50%;
        background-color: rgba(0, 0, 0, .6);
        cursor: pointer;
        opacity: 0;
        transition: ease opacity .2s;
    }

    .gallery:hover .step, .step:focus-visible {
        opacity: 1;
    }

    .previous {
        left: 8px;
    }

    .next {
        right: 8px;
    }

    .step img {
        height: 10px;
    }

    .caption {
        display: flex;
        justify-content: space-between;
        gap: 10px;
        margin-top: 5px;
        font-size: 12px;
        color: rgba(255, 255, 255, .5);
    }

    .viewer {
        position: fixed;
        inset: 0;
        z-index: 1000;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 10px;
        padding: 40px;
        border: none;
        background-color: rgba(0, 0, 0, .85);
        font-family: "Inter", sans-serif;
        font-size: 14px;
        color: white;
        cursor: zoom-out;
    }

    .viewer img {
        max-width: 100%;
        max-height: calc(100% - 30px);
        object-fit: contain;
        border-radius: 6px;
    }
</style>
