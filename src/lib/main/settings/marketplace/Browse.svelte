<script>
    import { createEventDispatcher, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import SettingWrapper from "../../../settings/SettingWrapper.svelte";
    import IconButtonSetting from "../../../settings/IconButtonSetting.svelte";
    import SmallButtonSetting from "../../../settings/SmallButtonSetting.svelte";
    import RippleLoader from "../../../common/RippleLoader.svelte";
    import TextSetting from "../../../settings/TextSetting.svelte";
    import ItemRow from "./ItemRow.svelte";
    import Tone from "./Tone.svelte";
    import { browseLine, itemTypes } from "./copy.js";

    export let client;
    export let options;
    export let type;
    export let query = "";

    const dispatch = createEventDispatcher();
    const { title, plural } = itemTypes[type];

    let result = null;
    let error = null;
    let busy = null;
    let request = 0;
    let timer;

    async function load(quiet = false) {
        const current = ++request;
        if (!quiet) {
            result = null;
            error = null;
        }

        try {
            const browsed = await invoke("browse_marketplace", { client, options, itemType: type, query });
            if (current === request) {
                result = browsed;
                error = null;
            }
        } catch (e) {
            console.error("Failed to browse the marketplace:", e);
            if (current === request) error = `${e}`;
        }
    }

    let searched = query;
    $: if (query !== searched) {
        searched = query;
        clearTimeout(timer);
        timer = setTimeout(load, 300);
    }

    async function install(item) {
        busy = item.id;
        try {
            await invoke("install_marketplace_item", { client, options, itemId: item.id, name: item.name, itemType: type });
        } catch (e) {
            console.error("Failed to install:", e);
            alert(`${e}`);
        }
        await load(true);
        busy = null;
    }

    onDestroy(() => clearTimeout(timer));
    load();
</script>

<div class="stack">
    <div class="back">
        <IconButtonSetting text="Back" icon="icon-prev" on:click={() => dispatch("back")} />
    </div>
    <TextSetting placeholder="Search {plural}" bind:value={query} on:keydown={e => e.key === "Escape" && (query = "")} />
</div>

<SettingWrapper {title} unbounded>
    <svelte:fragment slot="title-element">
        {#if type === "Addon" && result}
            <span class="aside">LiquidBounce {result.liquidbounce}</span>
        {/if}
    </svelte:fragment>
    {#if error}
        <div class="center">
            <div class="strong">Could not reach the marketplace.</div>
            <div class="note">{error}</div>
            <SmallButtonSetting text="Try again" on:click={() => load()} />
        </div>
    {:else if !result}
        <div class="center">
            <RippleLoader size={80} />
        </div>
    {:else}
        {#each result.items as item (item.id)}
            <ItemRow name={item.name} description={item.summary} dim={item.fit?.kind === "noVersion"} on:open={() => dispatch("open", item.id)}>
                {browseLine(item, result.liquidbounce)}
                <svelte:fragment slot="side">
                    {#if item.subscribed}
                        <span class="state"><Tone line={{ tone: "strong", text: "Installed" }} /></span>
                    {:else}
                        <SmallButtonSetting
                                text={busy === item.id ? "Installing" : "Install"}
                                disabled={busy !== null || !item.installable}
                                on:click={() => install(item)}
                        />
                    {/if}
                </svelte:fragment>
            </ItemRow>
        {:else}
            <div class="center">
                {#if query.trim()}
                    <div class="strong">No {plural} match “{query.trim()}”.</div>
                {:else}
                    <div class="strong">No {plural} published yet.</div>
                {/if}
                {#if query}
                    <div class="clear">
                        <IconButtonSetting text="Clear search" icon="icon-button-close" on:click={() => query = ""} />
                    </div>
                {/if}
            </div>
        {/each}
    {/if}
</SettingWrapper>

<style>
    .stack {
        display: flex;
        flex-direction: column;
        row-gap: 10px;
    }

    .back, .clear {
        display: flex;
    }

    .aside {
        font-size: 12px;
        line-height: 17px;
        color: rgba(255, 255, 255, .5);
    }

    .center {
        display: flex;
        flex-direction: column;
        align-items: center;
        row-gap: 5px;
        padding: 20px 0;
        text-align: center;
    }

    .strong {
        color: white;
    }

    .note {
        font-size: 12px;
        line-height: 15px;
        color: rgba(255, 255, 255, .5);
        word-break: break-word;
    }

    .clear {
        margin-top: 5px;
    }

    .state {
        font-size: 12px;
        white-space: nowrap;
    }
</style>
