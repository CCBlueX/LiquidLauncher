<script>
    import { createEventDispatcher, onDestroy, onMount, tick } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { confirm, open as dialogOpen } from "@tauri-apps/plugin-dialog";
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import SettingWrapper from "../../settings/SettingWrapper.svelte";
    import CustomModSetting from "../../settings/CustomModSetting.svelte";
    import IconButtonSetting from "../../settings/IconButtonSetting.svelte";
    import SmallButtonSetting from "../../settings/SmallButtonSetting.svelte";
    import RippleLoader from "../../common/RippleLoader.svelte";
    import ItemRow from "./client/ItemRow.svelte";
    import Tone from "./client/Tone.svelte";
    import Builds from "./client/Builds.svelte";
    import Browse from "./client/Browse.svelte";
    import Detail from "./client/Detail.svelte";
    import Modrinth from "./client/Modrinth.svelte";
    import { capitalize, modLine, noticeLine, removeQuestion, rowLine, selectionLine } from "./client/copy.js";
    import { track, untrack } from "./client/modrinth.js";

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

    $: build = versionState.currentBuild;

    let library = null;
    let error = null;
    let busy = null;
    let updating = new Set();
    let request = 0;

    $: notice = library ? noticeLine(library.notice) : null;
    $: sections = library ? [
        { title: "Add-ons", type: "Addon", rows: library.addons, empty: "No add-ons yet." },
        { title: "Themes", type: "Theme", rows: library.themes, empty: "No themes yet." },
        { title: "Scripts", type: "Script", rows: library.scripts, empty: "No scripts yet." }
    ] : [];

    async function load() {
        const current = ++request;
        try {
            const local = await invoke("get_marketplace_library", { client, options, check: false });
            if (current !== request) return;
            library = local;
            error = null;

            const checked = await invoke("get_marketplace_library", { client, options, check: true });
            if (current !== request) return;
            library = checked;
        } catch (e) {
            console.error("Failed to load the marketplace library:", e);
            if (current === request) error = `${e}`;
        }
    }

    // The main screen resolves the build again whenever the choice or the nightly builds change.
    let described;
    $: if (build !== described) {
        described = build;
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

    function undo(row, type) {
        return act(row, "install_marketplace_item", { name: row.name, itemType: type });
    }

    async function addFile() {
        try {
            const selected = await dialogOpen({
                directory: false,
                multiple: true,
                filters: [{ name: "", extensions: ["jar"] }],
                title: "Select a custom mod to install"
            });

            if (selected) {
                for (const file of selected) {
                    await invoke("install_custom_mod", {
                        branch: build.branch,
                        mcVersion: build.mcVersion,
                        path: file
                    });
                }

                dispatch("updateMods");
            }
        } catch (e) {
            console.error("Failed to install mod:", e);
            alert(`Failed to install mod: ${e}`);
        }
    }

    function deleteFile(fileName) {
        return invoke("delete_custom_mod", {
            branch: build.branch,
            mcVersion: build.mcVersion,
            modName: fileName
        });
    }

    async function removeMod(mod) {
        try {
            if (mod.source.type === "local") {
                await deleteFile(mod.source.fileName);
            } else {
                await untrack(options, build, mod.modrinth.projectId);
            }
        } catch (e) {
            console.error("Failed to delete mod:", e);
            alert(`Failed to delete mod: ${e}`);
        }
        dispatch("updateMods");
    }

    // A file Modrinth knows becomes a Modrinth mod on the newer version.
    async function updateMod(mod) {
        const key = modKey(mod);
        updating = new Set(updating).add(key);
        try {
            const entry = await invoke("modrinth_install", { client, options, projectId: mod.modrinth.projectId });
            await track(options, build, entry);
            if (mod.source.type === "local") {
                await deleteFile(mod.source.fileName);
            }
        } catch (e) {
            console.error("Failed to update mod:", e);
            alert(`${e}`);
        }
        updating.delete(key);
        updating = updating;
        dispatch("updateMods");
    }

    function modKey(mod) {
        return `${mod.source.type}:${mod.name}`;
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
        {@const selection = selectionLine(library.selection)}
        <div class="context openable" class:problem={notice.problem} role="status">
            <img src="img/icon/icon-version-lb.png" alt="">
            <button class="open" type="button" on:click={() => show({ name: "builds" })}>
                {#if library.liquidbounce}
                    <span class="build">LiquidBounce {library.liquidbounce}<span class="muted">{` · Minecraft ${library.minecraft}`}</span></span>
                {/if}
                {#if selection}
                    <span class="line">{selection}</span>
                {/if}
                {#if notice.text}
                    <span class="line"><Tone line={notice} /></span>
                {/if}
                {#if notice.detail}
                    <span class="line detail">{notice.detail}</span>
                {/if}
            </button>
            <div class="side">
                {#if notice.checking}
                    <RippleLoader size={30} />
                {:else if notice.retry}
                    <SmallButtonSetting text="Try again" on:click={load} />
                {/if}
                <img class="chevron" src="img/icon/icon-next.svg" alt="" aria-hidden="true">
            </div>
        </div>
    {:else if error}
        <div class="context problem" role="status">
            <img src="img/icon/icon-version-lb.png" alt="">
            <div class="line detail">{error}</div>
            <div class="side">
                <SmallButtonSetting text="Try again" on:click={load} />
            </div>
        </div>
    {/if}

    <SettingWrapper title="Recommended mods" unbounded>
        {#each versionState.recommendedMods as mod}
            <ToggleSetting
                    title={mod.name}
                    bind:value={mod.enabled}
                    disabled={mod.required}
                    on:change={() => dispatch("updateModStates")}
            />
        {/each}
    </SettingWrapper>

    {#if build}
        <SettingWrapper title="Additional mods - {capitalize(build.subsystem)} {build.mcVersion}" unbounded>
            <div slot="title-element" class="actions">
                <IconButtonSetting text="Add file" icon="icon-plus" on:click={addFile} />
                <IconButtonSetting text="Browse" icon="icon-plus" on:click={() => show({ name: "modrinth" })} />
            </div>
            {#each versionState.customMods as mod (modKey(mod))}
                <CustomModSetting
                        title={mod.title}
                        bind:value={mod.enabled}
                        lined={!!mod.modrinth}
                        on:change={() => dispatch("updateModStates")}
                        on:delete={() => removeMod(mod)}
                >
                    <svelte:fragment slot="line">
                        {#if mod.modrinth}
                            {@const { label, status } = modLine(mod.modrinth)}
                            {label}{#if status}{" · "}<Tone line={status} />{/if}
                        {/if}
                    </svelte:fragment>
                    <svelte:fragment slot="side">
                        {#if mod.modrinth?.update}
                            <SmallButtonSetting
                                    text={updating.has(modKey(mod)) ? "Updating" : "Update"}
                                    disabled={updating.has(modKey(mod))}
                                    on:click={() => updateMod(mod)}
                            />
                        {/if}
                    </svelte:fragment>
                </CustomModSetting>
            {:else}
                <div class="empty">No additional mods yet.</div>
            {/each}
        </SettingWrapper>
    {/if}

    {#each sections as section (section.type)}
        <SettingWrapper title={section.title} unbounded>
            <div slot="title-element">
                <IconButtonSetting text="Browse" icon="icon-plus" on:click={() => show({ name: "browse", type: section.type })} />
            </div>
            {#each section.rows as row (row.id)}
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
                            <SmallButtonSetting text="Undo" disabled={busy === row.id} on:click={() => undo(row, section.type)} />
                        {/if}
                    </svelte:fragment>
                </ItemRow>
            {:else}
                <div class="empty">{section.empty}</div>
            {/each}
        </SettingWrapper>
    {/each}
{:else if view.name === "builds"}
    <Builds
            {client}
            {options}
            on:back={() => show({ name: "list" })}
            on:select={e => selectBuild(e.detail)}
            on:updateData
    />
{:else if view.name === "modrinth"}
    <Modrinth
            {client}
            {options}
            {versionState}
            on:back={() => show({ name: "list" })}
            on:updateMods
            on:updateModStates
    />
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

    .context {
        position: relative;
        display: grid;
        grid-template-columns: 30px minmax(0, 1fr) max-content;
        column-gap: 10px;
        align-items: center;
        background-color: rgba(0, 0, 0, .26);
        border-radius: 6px;
        padding: 10px;
        border: solid 1px transparent;
        transition: ease border-color .2s;
    }

    .context.openable:hover, .context.openable:focus-within {
        border-color: #4677FF;
    }

    .context.problem {
        border-bottom-color: #B83529;
    }

    .context > img {
        width: 30px;
        height: 30px;
    }

    .open {
        display: grid;
        row-gap: 2px;
        text-align: left;
        background: transparent;
        border: none;
        padding: 0;
        color: white;
        font-family: "Inter", sans-serif;
        font-size: 14px;
        cursor: pointer;
        min-width: 0;
    }

    .open::after {
        content: "";
        position: absolute;
        inset: 0;
    }

    .muted {
        color: rgba(255, 255, 255, .5);
    }

    .line {
        font-size: 12px;
        line-height: 15px;
        color: rgba(255, 255, 255, .5);
    }

    .detail {
        word-break: break-word;
    }

    .side {
        position: relative;
        z-index: 1;
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .chevron {
        height: 10px;
        opacity: .5;
    }

    .actions {
        display: flex;
        column-gap: 15px;
    }

    .empty {
        font-size: 12px;
        color: rgba(255, 255, 255, .5);
    }
</style>
