<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { _ } from "svelte-i18n";
    import { shortDate } from "../../../../i18n.js";
    import ButtonSetting from "../../../settings/ButtonSetting.svelte";
    import ToggleSetting from "../../../settings/ToggleSetting.svelte";
    import ItemRow from "../../../settings/ItemRow.svelte";
    import ListView from "../../../settings/ListView.svelte";

    export let client;
    export let options;

    const dispatch = createEventDispatcher();

    let view;
    let result = null;
    let loadingMore = false;

    const request = () => invoke("request_build_page", { client, options, page: 1 });

    async function more() {
        const shown = result;
        loadingMore = true;
        try {
            const page = await invoke("request_build_page", { client, options, page: shown.page + 1 });
            if (result === shown) result = { ...page, builds: [...shown.builds, ...page.builds] };
        } catch (e) {
            console.error("Failed to request builds:", e);
            alert(`${e}`);
        } finally {
            loadingMore = false;
        }
    }

    function toggleNightly() {
        view.load();
        dispatch("updateData");
    }
</script>

<ListView
        bind:this={view}
        bind:result
        title={$_("builds.title")}
        {request}
        on:back
>
    <ToggleSetting
            slot="controls"
            title={$_("builds.showNightly")}
            bind:value={options.launcher.showNightlyBuilds}
            disabled={false}
            on:change={toggleNightly}
    />
    <svelte:fragment slot="aside">{result ? $_(result.nightly ? "builds.builds" : "builds.releases", { values: { count: result.total } }) : ""}</svelte:fragment>

    <ItemRow name={$_("builds.latest")} status={result.latest ? $_("common.selected") : null} on:open={() => dispatch("select", -1)} />
    {#each result.builds as build (build.buildId)}
        <ItemRow
                name={build.name}
                description={build.description}
                status={build.selected ? $_("common.selected") : null}
                on:open={() => dispatch("select", build.buildId)}
        >
            {$shortDate(build.date)} · {build.commit}
        </ItemRow>
    {/each}
    {#if result.page < result.pages}
        <div class="more">
            <ButtonSetting small text={loadingMore ? $_("common.loading") : $_("common.showMore")} disabled={loadingMore} on:click={more} />
        </div>
    {/if}
</ListView>

<style>
    .more {
        display: flex;
        justify-content: center;
        padding-top: 5px;
    }
</style>
