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

<SearchHeader placeholder="Search {plural}" bind:query on:back />

<SettingWrapper {title} unbounded>
    <svelte:fragment slot="title-element">
        {#if type === "Addon" && result}
            <span class="aside">LiquidBounce {result.liquidbounce}</span>
        {/if}
    </svelte:fragment>
    {#if error}
        <Message title="Could not reach the marketplace." note={error}>
            <SmallButtonSetting text="Try again" on:click={() => load()} />
        </Message>
    {:else if !result}
        <Message>
            <RippleLoader size={80} />
        </Message>
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
            <Message
                    title={query.trim() ? `No ${plural} match “${query.trim()}”.` : `No ${plural} published yet.`}
                    clearable={!!query}
                    on:clear={() => query = ""}
            />
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
