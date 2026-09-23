<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import SettingWrapper from "../../../settings/SettingWrapper.svelte";
    import IconButtonSetting from "../../../settings/IconButtonSetting.svelte";
    import ButtonSetting from "../../../settings/ButtonSetting.svelte";
    import SmallButtonSetting from "../../../settings/SmallButtonSetting.svelte";
    import RippleLoader from "../../../common/RippleLoader.svelte";
    import { count, versionTag } from "./copy.js";

    export let client;
    export let options;
    export let id;

    const dispatch = createEventDispatcher();

    let detail = null;
    let error = null;
    let busy = false;
    let previewFailed = false;

    async function load() {
        error = null;
        try {
            detail = await invoke("get_marketplace_item", { client, options, itemId: id });
        } catch (e) {
            console.error("Failed to load marketplace item:", e);
            error = `${e}`;
        }
    }

    async function install() {
        busy = true;
        try {
            await invoke("install_marketplace_item", {
                client,
                options,
                itemId: detail.id,
                name: detail.name,
                itemType: detail.type
            });
        } catch (e) {
            console.error("Failed to install:", e);
            alert(`${e}`);
        }
        await load();
        busy = false;
    }

    async function remove() {
        busy = true;
        try {
            await invoke("remove_marketplace_item", { options, itemId: detail.id });
            dispatch("back");
        } catch (e) {
            console.error("Failed to remove:", e);
            alert(`${e}`);
        } finally {
            busy = false;
        }
    }

    load();
</script>

<div class="stack">
    <div class="row-start">
        <IconButtonSetting text="Back" icon="icon-prev" on:click={() => dispatch("back")} />
    </div>
    {#if detail}
        {#if detail.preview && !previewFailed}
            <div class="preview">
                <img src={detail.preview} alt={detail.name} loading="lazy" on:error={() => previewFailed = true}>
            </div>
        {/if}
        <div class="head">
            <div>
                <div class="name">{detail.name}</div>
                <div class="meta">
                    {detail.type === "Addon" ? "Add-on" : "Theme"} by {detail.author} &middot; {count(detail.downloads, "download", "downloads")}
                </div>
            </div>
            {#if detail.subscribed}
                <IconButtonSetting text="Remove" icon="icon-button-close" on:click={() => !busy && remove()} />
            {/if}
        </div>
        {#if detail.summary}
            <div class="summary">{detail.summary}</div>
        {/if}
    {/if}
</div>

{#if detail}
    {#if !detail.subscribed}
        <ButtonSetting
                text={busy ? "Installing" : "Install"}
                color="#4677FF"
                disabled={busy || !detail.canInstall}
                on:click={install}
        />
    {/if}

    {#if detail.versions.length > 0}
        <SettingWrapper title="Versions" unbounded>
            <div class="versions">
                {#each detail.versions as version}
                    {@const tag = versionTag(version.tag)}
                    <span class="version" class:dim={tag?.dim}>{version.label}</span>
                    <span class="muted">{version.liquidbounce ?? ""}</span>
                    <span class="muted">{version.date ?? ""}</span>
                    <span class="tag" class:strong={tag?.strong}>
                        {tag?.text ?? ""}
                    </span>
                {/each}
            </div>
        </SettingWrapper>
    {/if}
{:else if error}
    <div class="center">
        <div class="strong">Could not reach the marketplace.</div>
        <div class="note">{error}</div>
        <SmallButtonSetting text="Try again" on:click={load} />
    </div>
{:else}
    <div class="center">
        <RippleLoader size={80} />
    </div>
{/if}

<style>
    .stack {
        display: flex;
        flex-direction: column;
        row-gap: 10px;
    }

    .row-start {
        display: flex;
    }

    .preview {
        height: 84px;
        border-radius: 6px;
        overflow: hidden;
        background-color: rgba(0, 0, 0, .26);
    }

    .preview img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        object-position: center top;
        display: block;
    }

    .head {
        display: grid;
        grid-template-columns: minmax(0, 1fr) max-content;
        column-gap: 10px;
        align-items: center;
    }

    .name {
        color: white;
        font-size: 16px;
    }

    .meta, .summary, .note, .muted {
        font-size: 12px;
        color: rgba(255, 255, 255, .5);
    }

    .meta {
        margin-top: 2px;
    }

    .summary {
        line-height: 1.4;
    }

    .versions {
        display: grid;
        grid-template-columns: max-content max-content max-content minmax(0, 1fr);
        column-gap: 15px;
        row-gap: 5px;
        align-items: baseline;
    }

    .version {
        color: white;
    }

    .version.dim {
        color: rgba(255, 255, 255, .5);
    }

    .tag {
        font-size: 12px;
        color: rgba(255, 255, 255, .5);
        text-align: right;
        white-space: nowrap;
    }

    .tag.strong, .strong {
        color: white;
    }

    .center {
        display: flex;
        flex-direction: column;
        align-items: center;
        row-gap: 5px;
        padding: 20px 0;
        text-align: center;
    }

    .note {
        line-height: 15px;
        word-break: break-word;
    }
</style>
