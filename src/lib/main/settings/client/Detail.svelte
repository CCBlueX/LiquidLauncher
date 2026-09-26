<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { confirm } from "@tauri-apps/plugin-dialog";
    import SettingWrapper from "../../../settings/SettingWrapper.svelte";
    import IconButtonSetting from "../../../settings/IconButtonSetting.svelte";
    import ButtonSetting from "../../../settings/ButtonSetting.svelte";
    import RippleLoader from "../../../common/RippleLoader.svelte";
    import ItemRow from "./ItemRow.svelte";
    import Message from "./Message.svelte";
    import { count, itemTypes, neededByLine, removeQuestion } from "./copy.js";

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
        const neededBy = detail.neededBy.map(dependent => dependent.name);
        if (neededBy.length > 0 && !await confirm(removeQuestion(detail.name, neededBy))) {
            return;
        }

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
                    {itemTypes[detail.type].name} by {detail.author} &middot; {count(detail.downloads, "download", "downloads")}
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
                disabled={busy || !detail.canInstall}
                on:click={install}
        />
    {/if}

    {#if detail.versions.length > 0}
        <SettingWrapper title="Versions" unbounded>
            <div class="versions">
                {#each detail.versions as { label, liquidbounce, date, tag }}
                    <span class="version" class:dim={tag?.kind === "notFor"}>{label}</span>
                    <span class="muted">{liquidbounce ?? ""}</span>
                    <span class="muted">{date ?? ""}</span>
                    <span class="tag" class:strong={tag?.kind === "installed"}>
                        {#if tag?.kind === "installed"}Installed{:else if tag?.kind === "notFor"}Not for {tag.liquidbounce}{/if}
                    </span>
                {/each}
            </div>
        </SettingWrapper>
    {/if}

    {#if detail.neededBy.length > 0}
        <SettingWrapper title="Needed by" unbounded>
            {#each detail.neededBy as dependent}
                <ItemRow name={dependent.name} openable={false}>{neededByLine(dependent)}</ItemRow>
            {/each}
        </SettingWrapper>
    {/if}
{:else if error}
    <Message title="Could not reach the marketplace." note={error}>
        <ButtonSetting small text="Try again" on:click={load} />
    </Message>
{:else}
    <Message>
        <RippleLoader size={80} />
    </Message>
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

    .meta, .summary, .muted {
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

    .tag.strong {
        color: white;
    }
</style>
