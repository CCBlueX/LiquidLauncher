<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { confirm } from "@tauri-apps/plugin-dialog";
    import { _, locale } from "svelte-i18n";
    import { shortDate } from "../../../../../i18n.js";
    import SettingWrapper from "../../../../settings/SettingWrapper.svelte";
    import IconButtonSetting from "../../../../settings/IconButtonSetting.svelte";
    import ButtonSetting from "../../../../settings/ButtonSetting.svelte";
    import ItemRow from "../../../../settings/ItemRow.svelte";
    import Message from "../../../../settings/Message.svelte";
    import RippleLoader from "../../../../common/RippleLoader.svelte";
    import Screenshots from "./Screenshots.svelte";

    export let client;
    export let options;
    export let id;

    const dispatch = createEventDispatcher();

    let detail = null;
    let error = null;
    let busy = false;

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
        const names = detail.neededBy.map(dependent => dependent.name);
        const question = names.length && $_("marketplace.removeQuestion", {
            values: {
                name: detail.name,
                count: names.length,
                names: new Intl.ListFormat($locale, { type: "conjunction" }).format(names)
            }
        });
        if (question && !await confirm(question)) {
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
        <IconButtonSetting text={$_("common.back")} icon="icon-prev" on:click={() => dispatch("back")} />
    </div>
    {#if detail}
        <div class="head">
            <div>
                <div class="name">{detail.name}</div>
                <div class="meta">
                    {$_("marketplace.byAuthor", { values: { type: $_("marketplace.type", { values: { type: detail.type } }), author: detail.author } })}
                    · {$_("marketplace.downloads", { values: { count: detail.downloads } })}
                </div>
            </div>
            {#if detail.subscribed}
                <IconButtonSetting text={$_("common.remove")} icon="icon-button-close" on:click={() => !busy && remove()} />
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
                text={busy ? $_("common.installing") : $_("common.install")}
                disabled={busy || !detail.canInstall}
                on:click={install}
        />
    {/if}

    {#if detail.screenshots.length > 0}
        <SettingWrapper title={$_("marketplace.screenshots")} unbounded>
            <Screenshots screenshots={detail.screenshots} />
        </SettingWrapper>
    {/if}

    {#if detail.versions.length > 0}
        <SettingWrapper title={$_("marketplace.versions")} unbounded>
            <div class="versions">
                {#each detail.versions as { version, liquidbounce, date, tag }}
                    <span class="version" class:dim={tag?.kind === "notFor"}>{version ?? $shortDate(date)}</span>
                    <span class="muted">{liquidbounce ?? ""}</span>
                    <span class="muted">{version && date ? $shortDate(date) : ""}</span>
                    <span class="tag" class:strong={tag?.kind === "installed"}>
                        {#if tag?.kind === "installed"}
                            {$_("common.installed")}
                        {:else if tag?.kind === "notFor"}
                            {$_("marketplace.notFor", { values: { version: tag.version } })}
                        {/if}
                    </span>
                {/each}
            </div>
        </SettingWrapper>
    {/if}

    {#if detail.neededBy.length > 0}
        <SettingWrapper title={$_("marketplace.neededBy")} unbounded>
            {#each detail.neededBy as dependent}
                {@const type = $_("marketplace.type", { values: { type: dependent.type } })}
                <ItemRow name={dependent.name} openable={false}>
                    {dependent.author ? $_("marketplace.byAuthor", { values: { type, author: dependent.author } }) : type}
                </ItemRow>
            {/each}
        </SettingWrapper>
    {/if}
{:else if error}
    <Message title={error}>
        <ButtonSetting small text={$_("common.tryAgain")} on:click={load} />
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
