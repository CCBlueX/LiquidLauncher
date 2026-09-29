<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
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
        placeholder="Search {title.toLowerCase()}"
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
                dim={item.unfit}
                status={item.subscribed ? "Installed" : null}
                on:open={() => dispatch("open", item.id)}
        >
            {item.line}
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
        <Message title={result.empty} clearable={!!query} on:clear={() => query = ""} />
    {/each}
</ListView>
