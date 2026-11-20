<script>
    import {createEventDispatcher} from "svelte";
    import {fly} from "svelte/transition";
    import TextSetting from "../settings/TextSetting.svelte";
    import Avatar from "./accounts/Avatar.svelte";
    import {accountTypeName, isActiveAccount} from "./accounts/accounts.js";

    export let account;
    export let accountList;

    const dispatch = createEventDispatcher();

    let root;
    let open = false;
    let search = "";

    $: shownAccounts = accountList.filter((entry) => entry.name.toLowerCase().includes(search.toLowerCase()));

    function toggleSwitcher() {
        open = !open;
        search = "";
        if (open) dispatch("openSwitcher");
    }

    function switchTo(entry) {
        open = false;
        if (!isActiveAccount(entry, account)) dispatch("switchAccount", entry);
    }

    function send(event) {
        open = false;
        dispatch(event);
    }

    function closeOnOutsideClick(event) {
        if (open && !root.contains(event.target)) open = false;
    }

    function closeOnEscape(event) {
        if (event.key === "Escape") open = false;
    }
</script>

<svelte:window on:click={closeOnOutsideClick} on:keydown={closeOnEscape} />

<div class="account" bind:this={root}>
    <div class="card" class:open>
        <button class="switch" type="button" aria-expanded={open} aria-label="Switch account" on:click={toggleSwitcher}>
            <Avatar uuid={account?.id} size={35} />
            <div class="details">
                <div class="username">{account?.name}</div>
                <div class="type">{accountTypeName(account)}</div>
            </div>
        </button>
        <button class="icon-button" type="button" aria-label="Manage accounts" title="Manage accounts" on:click={() => send("manageAccounts")}>
            <img src="img/icon/icon-pen.svg" alt="">
        </button>
        <button class="icon-button" type="button" aria-label="Settings" title="Settings" on:click={() => send("showSettings")}>
            <img src="img/icon/icon-cog.svg" alt="">
        </button>
    </div>

    {#if open}
        <div class="popover" role="menu" aria-label="Accounts" transition:fly={{y: -6, duration: 150}}>
            <div class="search">
                <TextSetting placeholder="Search..." bind:value={search} />
            </div>
            <div class="rows">
                {#each shownAccounts as entry (entry.id)}
                    <button class="row" class:active={isActiveAccount(entry, account)} type="button" role="menuitem" on:click={() => switchTo(entry)}>
                        <Avatar uuid={entry.uuid} size={28} />
                        <div class="row-details">
                            <span class="name">{entry.name}</span>
                            <span class="row-type">{entry.type}</span>
                        </div>
                    </button>
                {/each}
            </div>
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
        display: grid;
        grid-template-columns: auto max-content max-content;
        align-items: center;
        gap: 8px;
        min-width: 200px;
        height: 50px;
        padding: 8px 12px;
        border-radius: 6px;
        background-color: rgba(0, 0, 0, .36);
        transition: ease background-color .12s, ease border-radius .12s;
    }

    .card.open {
        background-color: rgba(0, 0, 0, .58);
        border-radius: 6px 6px 0 0;
    }

    .switch {
        display: flex;
        align-items: center;
        gap: 8px;
        min-width: 0;
        padding: 0;
        border: none;
        background-color: transparent;
        font-family: "Inter", sans-serif;
        text-align: left;
        cursor: pointer;
    }

    .details {
        font-size: 11px;
        font-weight: 600;
        min-width: 0;
    }

    .username {
        color: white;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .type {
        color: #45a049;
    }

    .icon-button {
        display: flex;
        align-items: center;
        padding: 0;
        border: none;
        background-color: transparent;
        cursor: pointer;
    }

    .icon-button img {
        height: 20px;
    }

    .popover {
        position: absolute;
        top: 100%;
        left: 0;
        right: 0;
        overflow: hidden;
        border-radius: 0 0 6px 6px;
        background-color: rgba(0, 0, 0, .58);
        backdrop-filter: blur(10px);
        box-shadow: 0 0 10px rgba(0, 0, 0, .5);
    }

    .search {
        padding: 8px 12px;
    }

    .rows {
        max-height: 300px;
        overflow-y: auto;
    }

    .row {
        display: flex;
        align-items: center;
        gap: 8px;
        width: 100%;
        padding: 6px 12px;
        border: none;
        background-color: transparent;
        font-family: "Inter", sans-serif;
        text-align: left;
        cursor: pointer;
        transition: ease background-color .12s;
    }

    .row:hover {
        background-color: rgba(255, 255, 255, .06);
    }

    .row-details {
        display: flex;
        flex-direction: column;
        min-width: 0;
    }

    .name {
        color: white;
        font-size: 12px;
        font-weight: 600;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .row.active .name {
        color: #4677FF;
    }

    .row-type {
        color: rgba(255, 255, 255, .5);
        font-size: 11px;
    }
</style>
