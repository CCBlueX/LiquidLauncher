<script>
    import { createEventDispatcher, onDestroy, onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import SettingWrapper from "../../../settings/SettingWrapper.svelte";
    import IconButtonSetting from "../../../settings/IconButtonSetting.svelte";
    import SmallButtonSetting from "../../../settings/SmallButtonSetting.svelte";
    import RippleLoader from "../../../common/RippleLoader.svelte";
    import ItemRow from "./ItemRow.svelte";
    import Tone from "./Tone.svelte";
    import { noticeLine, rowLine } from "./copy.js";

    export let client;
    export let options;

    const dispatch = createEventDispatcher();

    let library = null;
    let error = null;
    let busy = null;
    let request = 0;

    $: notice = library ? noticeLine(library.notice) : null;
    $: sections = library ? [
        { title: "Add-ons", type: "Addon", rows: library.addons, empty: "No add-ons yet." },
        { title: "Themes", type: "Theme", rows: library.themes, empty: "No themes yet." }
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

    function remove(row) {
        return act(row, "remove_marketplace_item", {});
    }

    function undo(row, type) {
        return act(row, "install_marketplace_item", { name: row.name, itemType: type });
    }

    let unlisten;
    onMount(async () => {
        unlisten = await listen("client-exited", load);
    });
    onDestroy(() => unlisten?.());

    load();
</script>

{#if library}
    <div class="context" class:problem={notice.problem} role="status">
        <img src="img/icon/icon-version-mc.png" alt="">
        <div>
            {#if library.minecraft}
                <div class="build">Minecraft {library.minecraft}<span class="muted">{` · LiquidBounce ${library.liquidbounce}`}</span></div>
            {/if}
            {#if notice.text}
                <div class="status"><Tone line={notice} /></div>
            {/if}
            {#if notice.detail}
                <div class="detail">{notice.detail}</div>
            {/if}
        </div>
        <div>
            {#if notice.checking}
                <RippleLoader size={30} />
            {:else if notice.retry}
                <SmallButtonSetting text="Try again" on:click={load} />
            {/if}
        </div>
    </div>

    {#each sections as section (section.type)}
        <SettingWrapper title={section.title} unbounded>
            <div slot="title-element">
                <IconButtonSetting text="Browse" icon="icon-plus" on:click={() => dispatch("browse", section.type)} />
            </div>
            {#each section.rows as row (row.id)}
                <ItemRow
                        name={row.name}
                        removable={!row.removed && busy !== row.id}
                        dim={row.removed || !!row.notFor}
                        on:open={() => dispatch("open", row.id)}
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
{:else if error}
    <div class="context problem" role="status">
        <img src="img/icon/icon-version-mc.png" alt="">
        <div class="detail">{error}</div>
        <div>
            <SmallButtonSetting text="Try again" on:click={load} />
        </div>
    </div>
{/if}

<style>
    .context {
        display: grid;
        grid-template-columns: 30px minmax(0, 1fr) max-content;
        column-gap: 10px;
        align-items: center;
        background-color: rgba(0, 0, 0, .26);
        border-radius: 6px;
        padding: 10px;
        border-bottom: solid 1px transparent;
    }

    .context.problem {
        border-color: #B83529;
    }

    .context > img {
        width: 30px;
        height: 30px;
    }

    .build {
        color: white;
    }

    .muted {
        color: rgba(255, 255, 255, .5);
    }

    .status, .detail {
        font-size: 12px;
        line-height: 15px;
        color: rgba(255, 255, 255, .5);
        margin-top: 2px;
    }

    .detail {
        margin-top: 3px;
        word-break: break-word;
    }

    .empty {
        font-size: 12px;
        color: rgba(255, 255, 255, .5);
    }
</style>
