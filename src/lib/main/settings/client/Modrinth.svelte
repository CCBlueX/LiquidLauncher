<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import ButtonSetting from "../../../settings/ButtonSetting.svelte";
    import ItemRow from "./ItemRow.svelte";
    import Message from "./Message.svelte";
    import SearchView from "./SearchView.svelte";
    import { capitalize, count } from "./copy.js";
    import { track } from "./modrinth.js";

    export let client;
    export let options;
    export let versionState;

    const dispatch = createEventDispatcher();
    const build = versionState.currentBuild;
    const states = { included: "Included", installed: "Installed" };

    let view;
    let query = "";
    let busy = new Set();

    const search = query => invoke("modrinth_search", { client, options, query });

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
        await view.load(true);
        busy.delete(hit.projectId);
        busy = busy;
    }
</script>

<SearchView
        bind:this={view}
        bind:query
        placeholder="Search Modrinth mods"
        title="Modrinth"
        failure="Could not reach Modrinth."
        {search}
        on:back
        let:result={hits}
>
    <svelte:fragment slot="aside">{capitalize(build.subsystem)} {build.mcVersion}</svelte:fragment>

    {#each hits as hit (hit.projectId)}
        <ItemRow name={hit.title} description={hit.description} openable={false} status={states[hit.state.kind]}>
            by {hit.author} &middot; {count(hit.downloads, "download", "downloads")}
            <svelte:fragment slot="side">
                {#if !states[hit.state.kind]}
                    <ButtonSetting
                            small
                            text={busy.has(hit.projectId) ? "Installing" : "Install"}
                            disabled={busy.has(hit.projectId)}
                            on:click={() => install(hit)}
                    />
                {/if}
            </svelte:fragment>
        </ItemRow>
    {:else}
        {#if query.trim()}
            <Message title="No mods match “{query.trim()}”." clearable on:clear={() => query = ""} />
        {/if}
    {/each}
</SearchView>
