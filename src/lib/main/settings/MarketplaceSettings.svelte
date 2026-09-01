<script>
    import {onMount} from "svelte";
    import SettingWrapper from "../../settings/SettingWrapper.svelte";
    import IconButtonSetting from "../../settings/IconButtonSetting.svelte";
    import RippleLoader from "../../common/RippleLoader.svelte";
    import {invoke} from "@tauri-apps/api/core";

    export let client;

    let subscribed = [];
    let available = [];
    let loading = true;
    let busyId = null;
    let error = null;

    $: subscribedIds = new Set(subscribed.map(item => item.id));

    async function load() {
        loading = true;
        error = null;

        try {
            [subscribed, available] = await Promise.all([
                invoke("get_marketplace_subscriptions"),
                invoke("browse_marketplace_items", {
                    client,
                    page: 1,
                    limit: 50,
                    query: null,
                    itemType: null
                }).then(response => response.items)
            ]);
        } catch (e) {
            console.error("Failed to load marketplace:", e);
            error = `${e}`;
        } finally {
            loading = false;
        }
    }

    async function subscribe(item) {
        busyId = item.id;
        error = null;

        try {
            await invoke("subscribe_marketplace_item", {
                client,
                itemId: item.id,
                name: item.name,
                itemType: item.type
            });
            subscribed = await invoke("get_marketplace_subscriptions");
        } catch (e) {
            console.error("Failed to subscribe:", e);
            error = `${e}`;
        } finally {
            busyId = null;
        }
    }

    async function unsubscribe(item) {
        busyId = item.id;
        error = null;

        try {
            await invoke("unsubscribe_marketplace_item", {itemId: item.id});
            subscribed = await invoke("get_marketplace_subscriptions");
        } catch (e) {
            console.error("Failed to unsubscribe:", e);
            error = `${e}`;
        } finally {
            busyId = null;
        }
    }

    onMount(load);
</script>

{#if error}
    <div class="error">{error}</div>
{/if}

<SettingWrapper title="Subscribed">
    {#if subscribed.length === 0}
        <div class="note">Nothing subscribed yet.</div>
    {:else}
        {#each subscribed as item}
            <div class="row">
                <div class="name">{item.name}<span class="type">{item.type}</span></div>
                <IconButtonSetting
                        text={busyId === item.id ? "Working" : "Unsubscribe"}
                        icon="icon-button-close"
                        on:click={() => busyId === null && unsubscribe(item)}
                />
            </div>
        {/each}
    {/if}
</SettingWrapper>

<SettingWrapper title="Available">
    <div slot="title-element">
        <IconButtonSetting text="Refresh" icon="icon-plus" on:click={load}/>
    </div>
    {#if loading}
        <div class="loader"><RippleLoader/></div>
    {:else}
        {#each available.filter(item => !subscribedIds.has(item.id)) as item}
            <div class="row">
                <div class="name">{item.name}<span class="type">{item.type}</span></div>
                <IconButtonSetting
                        text={busyId === item.id ? "Working" : "Subscribe"}
                        icon="icon-plus"
                        on:click={() => busyId === null && subscribe(item)}
                />
            </div>
        {:else}
            <div class="note">Nothing available.</div>
        {/each}
    {/if}
</SettingWrapper>

<div class="note">Add-ons are installed into the mods folder on the next launch.</div>

<style>
    .row {
        display: grid;
        grid-template-columns: 1fr max-content;
        align-items: center;
        gap: 10px;
    }

    .name {
        color: white;
        font-size: 14px;
    }

    .type {
        opacity: .5;
        margin-left: 8px;
        font-size: 12px;
    }

    .note {
        color: white;
        opacity: .5;
        font-size: 13px;
    }

    .error {
        color: #ff6b6b;
        font-size: 13px;
    }

    .loader {
        display: flex;
        justify-content: center;
    }
</style>
