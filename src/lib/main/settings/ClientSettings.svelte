<script>
    import { createEventDispatcher, onDestroy, onMount, tick } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { confirm } from "@tauri-apps/plugin-dialog";
    import SettingWrapper from "../../settings/SettingWrapper.svelte";
    import IconButtonSetting from "../../settings/IconButtonSetting.svelte";
    import ButtonSetting from "../../settings/ButtonSetting.svelte";
    import ItemRow from "../../settings/ItemRow.svelte";
    import Message from "../../settings/Message.svelte";
    import RippleLoader from "../../common/RippleLoader.svelte";
    import Builds from "./client/Builds.svelte";
    import Mods from "./client/mods/Mods.svelte";
    import Modrinth from "./client/mods/Modrinth.svelte";
    import Browse from "./client/marketplace/Browse.svelte";
    import Detail from "./client/marketplace/Detail.svelte";

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

    // The build that launches and the installed add-ons, themes and scripts, read from disk first and
    // then checked with the marketplace.
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
        if (row.removeQuestion && !await confirm(row.removeQuestion)) {
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
    {#if library}
        {@const { notice } = library}
        <SettingWrapper title="Build">
            <ItemRow
                    icon="img/icon/icon-version-lb.png"
                    name={library.liquidbounce ? `LiquidBounce ${library.liquidbounce} · Minecraft ${library.minecraft}` : notice.error}
                    description={library.selection}
                    on:open={() => show({ name: "builds" })}
            >
                {#if library.liquidbounce}
                    <span class="strong">{notice.error ?? notice.text ?? ""}</span>
                {/if}
                <svelte:fragment slot="side">
                    {#if notice.kind === "checking"}
                        <RippleLoader size={30} />
                    {:else if notice.kind === "offline"}
                        <ButtonSetting small text="Try again" on:click={load} />
                    {/if}
                </svelte:fragment>
            </ItemRow>
        </SettingWrapper>
    {:else if error}
        <SettingWrapper title="Build">
            <Message title={error}>
                <ButtonSetting small text="Try again" on:click={load} />
            </Message>
        </SettingWrapper>
    {/if}

    <Mods {client} {options} {versionState} on:browse={() => show({ name: "modrinth" })} on:updateMods on:updateModStates />

    {#if library}
        {#each library.sections as section (section.type)}
            <SettingWrapper title={section.title} unbounded>
                <div slot="title-element">
                    <IconButtonSetting text="Browse" icon="icon-plus" on:click={() => show({ name: "browse", section })} />
                </div>
                {#each section.items as row (row.id)}
                    <ItemRow
                            name={row.name}
                            removable={!row.removed && busy !== row.id}
                            dim={row.removed || row.unfit}
                            on:open={() => show({ name: "detail", id: row.id, from: view })}
                            on:remove={() => remove(row)}
                    >
                        {row.line}
                        <svelte:fragment slot="side">
                            {#if row.removed}
                                <ButtonSetting
                                        small
                                        text="Undo"
                                        disabled={busy === row.id}
                                        on:click={() => act(row, "install_marketplace_item", { name: row.name, itemType: section.type })}
                                />
                            {/if}
                        </svelte:fragment>
                    </ItemRow>
                {:else}
                    <div class="empty">{section.empty}</div>
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
            type={view.section.type}
            title={view.section.title}
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

    .strong {
        color: white;
        white-space: normal;
        word-break: break-word;
    }
</style>
