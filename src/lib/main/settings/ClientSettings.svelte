<script>
    import { createEventDispatcher, onDestroy, onMount, tick } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { confirm } from "@tauri-apps/plugin-dialog";
    import SettingWrapper from "../../settings/SettingWrapper.svelte";
    import IconButtonSetting from "../../settings/IconButtonSetting.svelte";
    import ButtonSetting from "../../settings/ButtonSetting.svelte";
    import BuildCard from "./client/BuildCard.svelte";
    import Mods from "./client/Mods.svelte";
    import ItemRow from "./client/ItemRow.svelte";
    import Builds from "./client/Builds.svelte";
    import Browse from "./client/Browse.svelte";
    import Detail from "./client/Detail.svelte";
    import Modrinth from "./client/Modrinth.svelte";
    import { itemTypes, removeQuestion, rowLine } from "./client/copy.js";

    export let client;
    export let options;
    export let versionState;

    const dispatch = createEventDispatcher();

    // Sub-views replace the list. Detail returns to where it was opened, Browse keeps its search and
    // the list its scroll position.
    let view = { name: "list" };
    let query = "";
    let anchor;
    let listScroll = 0;

    async function show(next) {
        const scroller = anchor.parentElement;
        if (view.name === "list") {
            listScroll = scroller.scrollTop;
        }
        if (next.name === "browse" && view.name !== "detail") {
            query = "";
        }
        if (next.name === "list") {
            load();
        }
        view = next;
        await tick();
        scroller.scrollTop = next.name === "list" ? listScroll : 0;
    }

    function selectBuild(buildId) {
        options.version.buildId = buildId;
        dispatch("updateData");
        show({ name: "list" });
    }

    // The installed add-ons, themes and scripts, read from disk first and then checked with the
    // marketplace.
    const sections = { Addon: "addons", Theme: "themes", Script: "scripts" };
    let library = null;
    let error = null;
    let busy = null;
    let request = 0;

    async function load() {
        const current = ++request;
        try {
            for (const check of [false, true]) {
                const loaded = await invoke("get_marketplace_library", { client, options, check });
                if (current !== request) return;
                library = loaded;
                error = null;
            }
        } catch (e) {
            console.error("Failed to load the marketplace library:", e);
            if (current === request) error = `${e}`;
        }
    }

    // The main screen fetches the build again whenever the choice or the nightly builds change.
    let described;
    $: if (versionState.currentBuild !== described) {
        described = versionState.currentBuild;
        load();
    }

    async function act(row, command, args) {
        busy = row.id;
        try {
            await invoke(command, { client, options, itemId: row.id, ...args });
        } catch (e) {
            console.error(`Failed to run ${command}:`, e);
            alert(`${e}`);
        } finally {
            busy = null;
        }
        await load();
    }

    async function remove(row) {
        if (row.neededBy.length > 0 && !await confirm(removeQuestion(row.name, row.neededBy))) {
            return;
        }
        await act(row, "remove_marketplace_item", {});
    }

    let unlisten;
    onMount(async () => {
        unlisten = await listen("client-exited", load);
    });
    onDestroy(() => unlisten?.());
</script>

<div class="anchor" bind:this={anchor}></div>

{#if view.name === "list"}
    <BuildCard {library} {error} on:open={() => show({ name: "builds" })} on:retry={load} />

    <Mods {client} {options} {versionState} on:browse={() => show({ name: "modrinth" })} on:updateMods on:updateModStates />

    {#if library}
        {#each Object.entries(sections) as [type, key] (type)}
            {@const { title, plural } = itemTypes[type]}
            <SettingWrapper {title} unbounded>
                <div slot="title-element">
                    <IconButtonSetting text="Browse" icon="icon-plus" on:click={() => show({ name: "browse", type })} />
                </div>
                {#each library[key] as row (row.id)}
                    <ItemRow
                            name={row.name}
                            removable={!row.removed && busy !== row.id}
                            dim={row.removed || !!row.notFor}
                            on:open={() => show({ name: "detail", id: row.id, from: view })}
                            on:remove={() => remove(row)}
                    >
                        {rowLine(row)}
                        <svelte:fragment slot="side">
                            {#if row.removed}
                                <ButtonSetting
                                        small
                                        text="Undo"
                                        disabled={busy === row.id}
                                        on:click={() => act(row, "install_marketplace_item", { name: row.name, itemType: type })}
                                />
                            {/if}
                        </svelte:fragment>
                    </ItemRow>
                {:else}
                    <div class="empty">No {plural} yet.</div>
                {/each}
            </SettingWrapper>
        {/each}
    {/if}
{:else if view.name === "builds"}
    <Builds {client} {options} on:back={() => show({ name: "list" })} on:select={e => selectBuild(e.detail)} on:updateData />
{:else if view.name === "modrinth"}
    <Modrinth {client} {options} {versionState} on:back={() => show({ name: "list" })} on:updateMods />
{:else if view.name === "browse"}
    <Browse
            {client}
            {options}
            type={view.type}
            bind:query
            on:back={() => show({ name: "list" })}
            on:open={e => show({ name: "detail", id: e.detail, from: view })}
    />
{:else}
    {#key view.id}
        <Detail {client} {options} id={view.id} on:back={() => show(view.from)} />
    {/key}
{/if}

<style>
    .anchor {
        display: none;
    }

    .empty {
        font-size: 12px;
        color: rgba(255, 255, 255, .5);
    }
</style>
