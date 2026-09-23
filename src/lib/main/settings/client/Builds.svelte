<script>
    import { createEventDispatcher, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import SettingWrapper from "../../../settings/SettingWrapper.svelte";
    import SmallButtonSetting from "../../../settings/SmallButtonSetting.svelte";
    import ToggleSetting from "../../../settings/ToggleSetting.svelte";
    import RippleLoader from "../../../common/RippleLoader.svelte";
    import ItemRow from "./ItemRow.svelte";
    import Message from "./Message.svelte";
    import SearchHeader from "./SearchHeader.svelte";
    import Tone from "./Tone.svelte";
    import { count, selectedLine } from "./copy.js";

    export let client;
    export let options;

    const dispatch = createEventDispatcher();

    let query = "";
    let result = null;
    let error = null;
    let loadingMore = false;
    let request = 0;
    let timer;

    async function load() {
        const current = ++request;
        result = null;
        error = null;

        try {
            const page = await invoke("request_build_page", { client, options, query, page: 1 });
            if (current === request) result = page;
        } catch (e) {
            console.error("Failed to request builds:", e);
            if (current === request) error = `${e}`;
        }
    }

    async function more() {
        const current = request;
        loadingMore = true;
        try {
            const page = await invoke("request_build_page", { client, options, query, page: result.page + 1 });
            if (current === request) result = { ...page, builds: [...result.builds, ...page.builds] };
        } catch (e) {
            console.error("Failed to request builds:", e);
            alert(`${e}`);
        } finally {
            loadingMore = false;
        }
    }

    let searched = query;
    $: if (query !== searched) {
        searched = query;
        clearTimeout(timer);
        timer = setTimeout(load, 300);
    }

    function toggleNightly() {
        load();
        dispatch("updateData");
    }

    onDestroy(() => clearTimeout(timer));
    load();
</script>

<SearchHeader placeholder="Search builds" bind:query on:back>
    <ToggleSetting
            title="Show nightly builds"
            bind:value={options.launcher.showNightlyBuilds}
            disabled={false}
            on:change={toggleNightly}
    />
</SearchHeader>

<SettingWrapper title="Builds" unbounded>
    <svelte:fragment slot="title-element">
        {#if result}
            <span class="aside">
                {options.launcher.showNightlyBuilds ? count(result.total, "build", "builds") : count(result.total, "release", "releases")}
            </span>
        {/if}
    </svelte:fragment>
    {#if error}
        <Message title="Could not reach the LiquidBounce API." note={error}>
            <SmallButtonSetting text="Try again" on:click={load} />
        </Message>
    {:else if !result}
        <Message>
            <RippleLoader size={80} />
        </Message>
    {:else}
        <ItemRow name="Latest" on:open={() => dispatch("select", -1)}>
            <svelte:fragment slot="side">
                {#if result.latest}
                    <span class="state"><Tone line={selectedLine} /></span>
                {/if}
            </svelte:fragment>
        </ItemRow>
        {#each result.builds as build (build.buildId)}
            <ItemRow name="{build.liquidbounce} · Minecraft {build.minecraft}" description={build.message}
                     on:open={() => dispatch("select", build.buildId)}>
                {build.date} &middot; {build.commit}
                <svelte:fragment slot="side">
                    {#if build.selected}
                        <span class="state"><Tone line={selectedLine} /></span>
                    {/if}
                </svelte:fragment>
            </ItemRow>
        {:else}
            {#if query.trim()}
                <Message
                        title="No builds match “{query.trim()}”."
                        clearable
                        on:clear={() => query = ""}
                />
            {/if}
        {/each}
        {#if result.page < result.pages}
            <div class="more">
                <SmallButtonSetting text={loadingMore ? "Loading" : "Show more"} disabled={loadingMore} on:click={more} />
            </div>
        {/if}
    {/if}
</SettingWrapper>

<style>
    .aside {
        font-size: 12px;
        line-height: 17px;
        color: rgba(255, 255, 255, .5);
    }

    .state {
        font-size: 12px;
        white-space: nowrap;
    }

    .more {
        display: flex;
        justify-content: center;
        padding-top: 5px;
    }
</style>
