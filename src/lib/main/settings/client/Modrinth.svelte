<script>
    import { createEventDispatcher, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import SettingWrapper from "../../../settings/SettingWrapper.svelte";
    import SmallButtonSetting from "../../../settings/SmallButtonSetting.svelte";
    import RippleLoader from "../../../common/RippleLoader.svelte";
    import ItemRow from "./ItemRow.svelte";
    import Message from "./Message.svelte";
    import SearchHeader from "./SearchHeader.svelte";
    import Tone from "./Tone.svelte";
    import { capitalize, count, hitState } from "./copy.js";
    import { track } from "./modrinth.js";

    export let client;
    export let options;
    export let versionState;

    const dispatch = createEventDispatcher();
    const build = versionState.currentBuild;
    const game = `${capitalize(build.subsystem)} ${build.mcVersion}`;

    let query = "";
    let hits = null;
    let error = null;
    let busy = new Set();
    let request = 0;
    let timer;

    async function load(quiet = false) {
        const current = ++request;
        if (!quiet) {
            hits = null;
            error = null;
        }

        try {
            const found = await invoke("modrinth_search", { client, options, query });
            if (current === request) {
                hits = found;
                error = null;
            }
        } catch (e) {
            console.error("Failed to search Modrinth:", e);
            if (current === request) error = `${e}`;
        }
    }

    let searched = query;
    $: if (query !== searched) {
        searched = query;
        clearTimeout(timer);
        timer = setTimeout(load, 300);
    }

    async function install(hit) {
        busy = new Set(busy).add(hit.projectId);
        try {
            if (hit.state.kind === "recommended") {
                const recommended = versionState.recommendedMods.find(mod => mod.name === hit.state.name);
                recommended.enabled = true;
                dispatch("updateModStates");
            } else {
                const entry = await invoke("modrinth_install", { client, options, projectId: hit.projectId });
                await track(options, build, entry);
                dispatch("updateMods");
            }
        } catch (e) {
            console.error("Failed to install:", e);
            alert(`${e}`);
        }
        await load(true);
        busy.delete(hit.projectId);
        busy = busy;
    }

    onDestroy(() => clearTimeout(timer));
    load();
</script>

<SearchHeader placeholder="Search Modrinth mods" bind:query on:back />

<SettingWrapper title="Modrinth" unbounded>
    <span slot="title-element" class="aside">{game}</span>
    {#if error}
        <Message title="Could not reach Modrinth." note={error}>
            <SmallButtonSetting text="Try again" on:click={() => load()} />
        </Message>
    {:else if !hits}
        <Message>
            <RippleLoader size={80} />
        </Message>
    {:else}
        {#each hits as hit (hit.projectId)}
            {@const state = hitState(hit.state)}
            <ItemRow name={hit.title} description={hit.description} openable={false}>
                by {hit.author} &middot; {count(hit.downloads, "download", "downloads")}
                <svelte:fragment slot="side">
                    {#if state}
                        <span class="state"><Tone line={state} /></span>
                    {:else}
                        <SmallButtonSetting
                                text={busy.has(hit.projectId) ? "Installing" : "Install"}
                                disabled={busy.has(hit.projectId)}
                                on:click={() => install(hit)}
                        />
                    {/if}
                </svelte:fragment>
            </ItemRow>
        {:else}
            {#if query.trim()}
                <Message
                        title="No mods match “{query.trim()}”."
                        clearable
                        on:clear={() => query = ""}
                />
            {/if}
        {/each}
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
</style>
