<script>
    import { fly } from "svelte/transition";
    import VirtualList from "./VirtualList.svelte";
    import { createEventDispatcher } from "svelte";
    import { confirm } from "@tauri-apps/plugin-dialog";
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import TextSetting from "../../settings/TextSetting.svelte";
    import ButtonSetting from "../../settings/ButtonSetting.svelte"
    import LogMessage from "./LogMessage.svelte";
    import { _ } from "svelte-i18n";

    export let messages;

    let autoScroll = true;
    let query = "";

    const filters = [
        { key: "error", levels: ["FATAL", "ERROR"], color: "#ff4444", shown: true },
        { key: "warn", levels: ["WARN"], color: "#ff9800", shown: true },
        { key: "info", levels: ["INFO"], color: "#60b675", shown: true },
        { key: "debug", levels: ["DEBUG", "TRACE"], color: "rgba(255, 255, 255, 0.7)", shown: true },
    ];

    $: pattern = query ? new RegExp("(" + query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + ")", "i") : null;
    $: levels = filters.filter(filter => filter.shown).flatMap(filter => filter.levels);
    $: shown = pattern || filters.some(filter => !filter.shown) ? filterLines(messages, levels, pattern) : messages;

    // Text lines such as a stack trace go with the event above them, output before the first event counts as info.
    // A search keeps the event of each matching line, and all of an event whose first line matches.
    function filterLines(lines, levels, pattern) {
        const shown = [];
        let entry = [];
        const flush = () => {
            const [first, ...rest] = entry;
            if (!levels.includes(first.level ?? "INFO")) return;
            if (!pattern || pattern.test(first.text)) {
                shown.push(...entry);
                return;
            }
            const matches = rest.filter(line => pattern.test(line.text));
            if (matches.length) shown.push(first, ...matches);
        };
        for (const line of lines) {
            if (line.kind !== "text" && entry.length) {
                flush();
                entry = [];
            }
            entry.push(line);
        }
        if (entry.length) flush();
        return shown;
    }

    const dispatch = createEventDispatcher();

    async function handleUploadSetting(e) {
        const log = messages.map(line => line.text).join("\n");

        if (await confirm($_("log.uploadConfirm")) !== true) {
            return;
        }

        const response = await fetch("https://logs.liquidbounce.net/api/1/log", {
            body: new URLSearchParams({ content: log }),
            method: "POST"
        });

        const responseData = await response.json().catch(() => ({ success: false, error: `HTTP ${response.status}` }));

        if (!response.ok || !responseData.success) {
            alert($_("log.uploadFailed", { values: { error: `${responseData.error}` } }));
            return;
        }

        prompt($_("log.uploaded"), responseData.url);
    }
</script>

<div class="log" transition:fly={{ y: -10, duration: 200 }}>
    <div class="header">
        <div class="title">{$_("log.title")}</div>
        <div class="filters">
            <div class="search">
                <TextSetting placeholder={$_("log.search")} bind:value={query} />
            </div>
            {#each filters as filter}
                <label class="filter" style:--color={filter.color}>
                    <input type="checkbox" bind:checked={filter.shown}>
                    <span>{$_(`log.filters.${filter.key}`)}</span>
                </label>
            {/each}
        </div>
        <button class="button-hide" on:click={() => dispatch("hideClientLog")}>
            <img class="icon" src="img/icon/icon-button-close.svg" alt={$_("common.close")}>
        </button>
    </div>

    <div class="output">
        <!-- Starts over on a new filter, as the old row heights belong to other lines. -->
        {#key `${levels}|${query}`}
            <VirtualList items={shown} let:item {autoScroll}>
                <LogMessage line={item} {pattern} />
            </VirtualList>
        {/key}
    </div>

    <div class="settings">
        <ToggleSetting title={$_("log.autoScroll")} disabled={false} bind:value={autoScroll} />
        <ButtonSetting text={$_("log.upload")} color="#4677FF" small on:click={handleUploadSetting}></ButtonSetting>
    </div>
</div>

<style>
    .log {
        width: calc(100% - 150px);
        height: calc(100% - 150px);
        position: fixed;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        background-color: rgba(0, 0, 0, 0.58);
        backdrop-filter: blur(10px);
        padding: 25px;
        border-radius: 6px;
        z-index: 1000;
        display: flex;
        flex-direction: column;
        box-shadow: 0 0 10px rgba(0, 0, 0, 0.5);
    }

    .settings {
        display: flex;
        gap: 20px;
        align-items: center;
        justify-content: flex-end;
    }

    .output {
        flex: 1;
        overflow: hidden;
        margin-bottom: 10px;
    }

    .header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 20px;
    }

    .title {
        font-size: 16px;
        color: white;
    }

    .filters {
        display: flex;
        align-items: center;
        gap: 6px;
        margin-left: auto;
        margin-right: 20px;
    }

    .search {
        width: 200px;
        margin-right: 6px;
    }

    .filter {
        position: relative;
    }

    .filter input {
        position: absolute;
        opacity: 0;
        pointer-events: none;
    }

    .filter span {
        display: block;
        padding: 3px 8px;
        border: solid 1px rgba(255, 255, 255, 0.15);
        border-radius: 4px;
        color: rgba(255, 255, 255, 0.45);
        font-size: 12px;
        cursor: pointer;
    }

    .filter input:checked + span {
        color: var(--color);
        border-color: var(--color);
    }

    .filter input:focus-visible + span {
        outline: solid 1px #4677FF;
    }

    .button-hide {
        background-color: transparent;
        border: none;
        cursor: pointer;
    }
</style>
