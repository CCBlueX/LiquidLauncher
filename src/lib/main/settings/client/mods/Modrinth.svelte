<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import ButtonSetting from "../../../../settings/ButtonSetting.svelte";
    import ItemRow from "../../../../settings/ItemRow.svelte";
    import Message from "../../../../settings/Message.svelte";
    import ListView from "../../../../settings/ListView.svelte";
    import { track } from "./Mods.svelte";

    export let client;
    export let options;
    export let versionState;

    const dispatch = createEventDispatcher();
    const build = versionState.currentBuild;

    let view;
    let query = "";
    let busy = new Set();

    const request = query => invoke("modrinth_search", { client, options, query });

    async function install(hit) {
        busy = new Set(busy).add(hit.projectId);
        try {
            const entry = await invoke("modrinth_install", { client, options, projectId: hit.projectId });
            await track(options, build, entry);
            dispatch("updateMods");
        } catch (e) {
            console.error("Failed to install:", e);
            alert(`${e}`);
        }
        await view.load(true);
        busy.delete(hit.projectId);
        busy = busy;
    }
</script>

<ListView
        bind:this={view}
        bind:query
        placeholder="Search Modrinth mods"
        title="Modrinth"
        {request}
        on:back
        let:result
>
    <svelte:fragment slot="aside" let:result>{result?.target ?? ""}</svelte:fragment>

    {#each result.hits as hit (hit.projectId)}
        <ItemRow name={hit.title} description={hit.description} openable={false} status={hit.installed ? "Installed" : null}>
            {hit.line}
            <svelte:fragment slot="side">
                {#if !hit.installed}
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
        {#if result.empty}
            <Message title={result.empty} clearable on:clear={() => query = ""} />
        {/if}
    {/each}
</ListView>
