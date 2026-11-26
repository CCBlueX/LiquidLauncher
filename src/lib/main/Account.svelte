<script>
    import {createEventDispatcher} from "svelte";
    import {fly} from "svelte/transition";
    import {createPopover} from "@melt-ui/svelte";
    import TextSetting from "../settings/TextSetting.svelte";
    import Avatar from "../accounts/Avatar.svelte";
    import AccountRow from "../accounts/AccountRow.svelte";

    export let account;
    export let accounts;
    /** A running client writes the list when it exits, so it cannot be edited until then. */
    export let locked;

    const dispatch = createEventDispatcher();
    const duration = matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 150;

    // The stylesheet places the popover under the card, it stays where it is in the document.
    const {
        elements: {trigger, content},
        states: {open},
    } = createPopover({forceVisible: true, positioning: null, portal: null});

    let search = "";

    $: query = search.trim().toLowerCase();
    $: shown = accounts.filter(entry => entry.name.toLowerCase().includes(query));
    $: if (!$open) search = "";

    function choose(entry) {
        $open = false;
        if (!entry.active) dispatch("switchAccount", entry);
    }

    function addAccount() {
        $open = false;
        dispatch("addAccount");
    }
</script>

<div class="account">
    <div class="card" class:open={$open}>
        <button class="switch" type="button" title="Switch account" {...$trigger} use:trigger>
            <Avatar uuid={account?.id} size={35} />
            <span class="details">
                <span class="username">{account?.name}</span>
                <!-- options.json says "Premium" for a Microsoft account. -->
                <span class="type">{account?.type === "Premium" ? "Microsoft" : account?.type}</span>
            </span>
            <img class="chevron" src="img/icon/icon-next.svg" alt="">
        </button>
        <button class="settings" type="button" title="Settings" aria-label="Settings" on:click={() => dispatch("showSettings")}>
            <img src="img/icon/icon-cog.svg" alt="">
        </button>
    </div>

    {#if $open}
        <div class="popover" {...$content} use:content transition:fly={{y: -6, duration}}>
            <!-- Few enough to see at a glance otherwise. -->
            {#if accounts.length > 5}
                <TextSetting placeholder="Search accounts" bind:value={search} />
            {/if}
            <div class="rows">
                {#each shown as entry (entry.id)}
                    <AccountRow {entry} on:click={() => choose(entry)}>
                        {#if !locked && !entry.active}
                            <button class="remove" type="button" title="Remove" aria-label="Remove {entry.name}" on:click={() => dispatch("removeAccount", entry)}>
                                <img src="img/icon/icon-trash.svg" alt="">
                            </button>
                        {/if}
                    </AccountRow>
                {/each}
            </div>
            <button class="add" type="button" disabled={locked} on:click={addAccount}>
                <img src="img/icon/icon-plus.svg" alt="">
                Add account
            </button>
            {#if locked}
                <p class="note">Close LiquidBounce to add or remove accounts.</p>
            {/if}
        </div>
    {/if}
</div>

<style>
    .account {
        position: relative;
        z-index: 10;
        flex: none;
    }

    .card {
        display: flex;
        align-items: center;
        column-gap: 12px;
        min-width: 220px;
        height: 50px;
        padding: 8px 12px;
        border-radius: 6px;
        background-color: rgba(0, 0, 0, .36);
        transition: ease background-color .2s;
    }

    .card.open {
        background-color: rgba(0, 0, 0, .58);
        border-radius: 6px 6px 0 0;
    }

    .switch {
        flex: 1;
        display: flex;
        align-items: center;
        column-gap: 8px;
        min-width: 0;
        padding: 0;
        border: none;
        background: none;
        font-family: "Inter", sans-serif;
        text-align: left;
        cursor: pointer;
    }

    .details {
        flex: 1;
        display: grid;
        min-width: 0;
        font-size: 11px;
        font-weight: 600;
    }

    .username {
        color: white;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .type {
        color: rgba(255, 255, 255, .5);
    }

    .chevron {
        height: 10px;
        opacity: .5;
        transform: rotate(90deg);
        transition: ease transform .2s;
    }

    .open .chevron {
        transform: rotate(-90deg);
    }

    .settings {
        display: flex;
        padding: 0;
        border: none;
        background: none;
        cursor: pointer;
    }

    .popover {
        position: absolute;
        top: 100%;
        left: 0;
        right: 0;
        display: grid;
        row-gap: 6px;
        padding: 6px;
        border-radius: 0 0 6px 6px;
        background-color: rgba(0, 0, 0, .58);
        backdrop-filter: blur(10px);
        box-shadow: 0 10px 10px rgba(0, 0, 0, .5);
    }

    .rows {
        display: grid;
        row-gap: 2px;
        max-height: 300px;
        overflow-y: auto;
    }

    .remove {
        position: relative;
        display: flex;
        padding: 0;
        border: none;
        background: none;
        cursor: pointer;
        opacity: 0;
        transition: ease opacity .2s;
    }

    .remove img {
        height: 16px;
    }

    :global(.row:hover) > .remove,
    :global(.row:focus-within) > .remove {
        opacity: 1;
    }

    .add {
        display: flex;
        align-items: center;
        column-gap: 10px;
        padding: 8px 10px;
        border: none;
        border-radius: 6px;
        background: none;
        color: white;
        font-family: "Inter", sans-serif;
        font-size: 12px;
        cursor: pointer;
        transition: ease background-color .2s;
    }

    .add:hover {
        background-color: rgba(255, 255, 255, .06);
    }

    .add:disabled {
        opacity: .5;
        pointer-events: none;
    }

    .add img {
        height: 10px;
    }

    .note {
        padding: 0 10px 6px;
        color: rgba(255, 255, 255, .5);
        font-size: 11px;
        line-height: 14px;
    }

    .switch:focus-visible,
    .settings:focus-visible,
    .remove:focus-visible,
    .add:focus-visible {
        outline: solid 2px white;
        outline-offset: 2px;
    }
</style>
