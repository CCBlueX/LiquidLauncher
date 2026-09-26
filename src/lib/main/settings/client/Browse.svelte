<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import ButtonSetting from "../../../settings/ButtonSetting.svelte";
    import ItemRow from "./ItemRow.svelte";
    import Message from "./Message.svelte";
    import ListView from "./ListView.svelte";
    import { browseLine, itemTypes } from "./copy.js";

    export let client;
    export let options;
    export let type;
    export let query = "";

    const dispatch = createEventDispatcher();
    const { title, plural } = itemTypes[type];

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
        placeholder="Search {plural}"
        {title}
        failure="Could not reach the marketplace."
        {request}
        on:back
        let:result
>
    <svelte:fragment slot="aside" let:result>
        {#if type === "Addon" && result}LiquidBounce {result.liquidbounce}{/if}
    </svelte:fragment>

    {#each result.items as item (item.id)}
        <ItemRow
                name={item.name}
                description={item.summary}
                dim={item.fit?.kind === "noVersion"}
                status={item.subscribed ? "Installed" : null}
                on:open={() => dispatch("open", item.id)}
        >
            {browseLine(item, result.liquidbounce)}
            <svelte:fragment slot="side">
                {#if !item.subscribed}
                    <ButtonSetting
                            small
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
</ListView>
