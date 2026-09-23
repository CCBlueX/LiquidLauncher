<script>
    import { tick } from "svelte";
    import Library from "./marketplace/Library.svelte";
    import Browse from "./marketplace/Browse.svelte";
    import Detail from "./marketplace/Detail.svelte";

    export let client;
    export let options;

    // Detail returns to the browse list it was opened from, with its search kept.
    let view = { name: "library" };
    let query = "";
    let anchor;

    async function show(next) {
        if (next.name === "browse" && view.name !== "detail") {
            query = "";
        }
        view = next;
        await tick();
        anchor.parentElement.scrollTop = 0;
    }
</script>

<div class="anchor" bind:this={anchor}></div>

{#if view.name === "library"}
    <Library
            {client}
            {options}
            on:browse={e => show({ name: "browse", type: e.detail })}
            on:open={e => show({ name: "detail", id: e.detail, from: view })}
    />
{:else if view.name === "browse"}
    <Browse
            {client}
            {options}
            type={view.type}
            bind:query
            on:back={() => show({ name: "library" })}
            on:open={e => show({ name: "detail", id: e.detail, from: view })}
    />
{:else}
    {#key view.id}
        <Detail
                {client}
                {options}
                id={view.id}
                on:back={() => show(view.from)}
        />
    {/key}
{/if}

<style>
    .anchor {
        display: none;
    }
</style>
