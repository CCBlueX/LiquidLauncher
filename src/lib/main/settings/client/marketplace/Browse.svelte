<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { _ } from "svelte-i18n";
    import { shortDate } from "../../../../../i18n.js";
    import ButtonSetting from "../../../../settings/ButtonSetting.svelte";
    import ItemRow from "../../../../settings/ItemRow.svelte";
    import Message from "../../../../settings/Message.svelte";
    import ListView from "../../../../settings/ListView.svelte";

    export let client;
    export let options;
    export let type;
    export let title;
    export let query = "";

    const dispatch = createEventDispatcher();

    let view;
    let busy = null;

    const request = query => invoke("browse_marketplace", { client, options, itemType: type, query });

    async function install(item) {
        busy = item.id;
        try {
            await invoke("install_marketplace_item", { client, options, itemId: item.id, name: item.name, itemType: type });
        } catch (e) {
            console.error("Failed to install:", e);
            alert(`${e}`);
        }
        await view.load(true);
        busy = null;
    }
</script>

<ListView
        bind:this={view}
        bind:query
        placeholder={$_("marketplace.search", { values: { type } })}
        {title}
        {request}
        on:back
        let:result
>
    <svelte:fragment slot="aside" let:result>{result?.aside ?? ""}</svelte:fragment>

    {#each result.items as item (item.id)}
        <ItemRow
                name={item.name}
                description={item.summary}
                dim={!!item.notFor}
                status={item.subscribed ? $_("common.installed") : null}
                on:open={() => dispatch("open", item.id)}
        >
            {[
                item.version,
                item.liquidbounce,
                item.notFor && $_("marketplace.notFor", { values: { version: item.notFor } }),
                item.updated && $shortDate(item.updated),
                $_("marketplace.downloads", { values: { count: item.downloads } })
            ].filter(Boolean).join(" · ")}
            <svelte:fragment slot="side">
                {#if !item.subscribed}
                    <ButtonSetting
                            small
                            text={busy === item.id ? $_("common.installing") : $_("common.install")}
                            disabled={busy !== null || !item.installable}
                            on:click={() => install(item)}
                    />
                {/if}
            </svelte:fragment>
        </ItemRow>
    {:else}
        <Message
                title={result.query
                    ? $_("marketplace.noMatch", { values: { type, query: result.query } })
                    : $_("marketplace.nonePublished", { values: { type } })}
                clearable={!!query}
                on:clear={() => query = ""}
        />
    {/each}
</ListView>
