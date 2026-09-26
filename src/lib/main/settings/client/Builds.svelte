<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import ButtonSetting from "../../../settings/ButtonSetting.svelte";
    import ToggleSetting from "../../../settings/ToggleSetting.svelte";
    import ItemRow from "./ItemRow.svelte";
    import ListView from "./ListView.svelte";
    import { count } from "./copy.js";

    export let client;
    export let options;

    const dispatch = createEventDispatcher();

    let view;
    let result = null;
    let loadingMore = false;

    const request = () => invoke("request_build_page", { client, options, page: 1 });

    async function more() {
        const shown = result;
        loadingMore = true;
        try {
            const page = await invoke("request_build_page", { client, options, page: shown.page + 1 });
            if (result === shown) result = { ...page, builds: [...shown.builds, ...page.builds] };
        } catch (e) {
            console.error("Failed to request builds:", e);
            alert(`${e}`);
        } finally {
            loadingMore = false;
        }
    }

    function toggleNightly() {
        view.load();
        dispatch("updateData");
    }
</script>

<ListView
        bind:this={view}
        bind:result
        title="Builds"
        failure="Could not reach the LiquidBounce API."
        {request}
        on:back
>
    <ToggleSetting
            slot="controls"
            title="Show nightly builds"
            bind:value={options.launcher.showNightlyBuilds}
            disabled={false}
            on:change={toggleNightly}
    />
    <svelte:fragment slot="aside">
        {#if result}
            {options.launcher.showNightlyBuilds ? count(result.total, "build", "builds") : count(result.total, "release", "releases")}
        {/if}
    </svelte:fragment>

    <ItemRow name="Latest" status={result.latest ? "Selected" : null} on:open={() => dispatch("select", -1)} />
    {#each result.builds as build (build.buildId)}
        <ItemRow
                name="{build.liquidbounce} · Minecraft {build.minecraft}"
                description={build.message}
                status={build.selected ? "Selected" : null}
                on:open={() => dispatch("select", build.buildId)}
        >
            {build.date} &middot; {build.commit}
        </ItemRow>
    {/each}
    {#if result.page < result.pages}
        <div class="more">
            <ButtonSetting small text={loadingMore ? "Loading" : "Show more"} disabled={loadingMore} on:click={more} />
        </div>
    {/if}
</ListView>

<style>
    .more {
        display: flex;
        justify-content: center;
        padding-top: 5px;
    }
</style>
