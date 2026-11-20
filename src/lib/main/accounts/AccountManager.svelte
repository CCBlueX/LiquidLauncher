<script>
    import {createEventDispatcher, onMount} from "svelte";
    import {fly} from "svelte/transition";
    import {invoke} from "@tauri-apps/api/core";
    import SettingsContainer from "../../settings/SettingsContainer.svelte";
    import TextSetting from "../../settings/TextSetting.svelte";
    import ButtonSetting from "../../settings/ButtonSetting.svelte";
    import AccountRow from "./AccountRow.svelte";
    import AddAccount from "./AddAccount.svelte";
    import {accounts, hasMicrosoftAccount, isActiveAccount, loadAccounts} from "./accounts.js";

    export let options;

    const dispatch = createEventDispatcher();
    const ACCOUNT_TYPES = ["Microsoft", "Offline", "Session"];

    let search = "";
    let shownTypes = [...ACCOUNT_TYPES];
    let typeMenu;
    let typeMenuOpen = false;
    let addShown = false;
    let offlineAvailable = false;

    $: shownAccounts = $accounts.filter((entry) =>
        shownTypes.includes(entry.type) && entry.name.toLowerCase().includes(search.toLowerCase())
    );
    $: canAddOtherTypes = offlineAvailable || hasMicrosoftAccount($accounts);

    function toggleType(type) {
        if (shownTypes.includes(type)) {
            shownTypes = shownTypes.filter((shown) => shown !== type);
        } else {
            shownTypes = [...shownTypes, type];
        }
    }

    function closeTypeMenuOnOutsideClick(event) {
        if (typeMenuOpen && !typeMenu.contains(event.target)) typeMenuOpen = false;
    }

    async function toggleFavorite(entry) {
        try {
            await invoke("set_favorite", {options, id: entry.id, favorite: !entry.favorite});
        } catch (error) {
            alert("Failed to update favorite.\n\n" + error);
        }
        await loadAccounts(options);
    }

    async function removeAccount(entry) {
        try {
            await invoke("remove_account", {options, id: entry.id});
        } catch (error) {
            alert("Failed to delete account.\n\n" + error);
            return;
        }
        await loadAccounts(options);

        // Without a Microsoft account left, the launcher signs out.
        if (entry.type === "Microsoft" && !hasMicrosoftAccount($accounts)) {
            options.start.account = null;
            options.store();
        }
    }

    async function accountAdded() {
        addShown = false;
        await loadAccounts(options);
    }

    onMount(async () => {
        loadAccounts(options);
        offlineAvailable = await invoke("offline_login_available");
    });
</script>

<svelte:window on:click={closeTypeMenuOnOutsideClick} />

{#if addShown}
    <AddAccount
            bind:options
            {canAddOtherTypes}
            on:added={accountAdded}
            on:close={() => addShown = false}
    />
{:else}
    <SettingsContainer title="Accounts" pinned on:hideSettings={() => dispatch("hide")}>
        <div class="filters" slot="tabs">
            <div class="search">
                <TextSetting placeholder="Search..." bind:value={search} />
            </div>
            <div class="select" bind:this={typeMenu}>
                <button class="select-head" class:open={typeMenuOpen} type="button" aria-haspopup="listbox" aria-expanded={typeMenuOpen} on:click={() => typeMenuOpen = !typeMenuOpen}>Account Type</button>
                {#if typeMenuOpen}
                    <div class="options" role="listbox" aria-multiselectable="true" aria-label="Account Type" transition:fly={{y: -4, duration: 120}}>
                        {#each ACCOUNT_TYPES as type}
                            <button
                                    class="option"
                                    class:selected={shownTypes.includes(type)}
                                    type="button"
                                    role="option"
                                    aria-selected={shownTypes.includes(type)}
                                    on:click={() => toggleType(type)}
                            >{type}</button>
                        {/each}
                    </div>
                {/if}
            </div>
        </div>

        <div class="rows">
            {#each shownAccounts as entry, index (entry.id)}
                <AccountRow
                        {entry}
                        active={isActiveAccount(entry, options.start.account)}
                        tooltipsBelow={index === 0}
                        on:play={() => dispatch("switchAccount", entry)}
                        on:favorite={() => toggleFavorite(entry)}
                        on:remove={() => removeAccount(entry)}
                />
            {:else}
                <p class="empty">No accounts found.</p>
            {/each}
        </div>

        <div class="footer" slot="footer">
            <ButtonSetting text="Add account" on:click={() => addShown = true} />
        </div>
    </SettingsContainer>
{/if}

<style>
    .filters {
        display: flex;
        column-gap: 10px;
        margin-bottom: 10px;
    }

    .search {
        flex: 1;
        min-width: 0;
    }

    .select {
        position: relative;
        width: 150px;
    }

    /* Same look as SelectSetting. */
    .select-head {
        width: 100%;
        background-color: rgba(0, 0, 0, .26);
        border: none;
        border-bottom: solid 1px #4677FF;
        color: white;
        font-family: "Inter", sans-serif;
        text-align: left;
        padding: 5px;
        border-radius: 3px;
        cursor: pointer;
    }

    .select-head.open {
        border-radius: 3px 3px 0 0;
    }

    .options {
        position: absolute;
        top: 100%;
        left: 0;
        width: 100%;
        border-radius: 0 0 3px 3px;
        background-color: rgba(0, 0, 0, .9);
        box-shadow: 0 6px 10px rgba(0, 0, 0, .4);
        z-index: 5;
    }

    .option {
        display: block;
        width: 100%;
        padding: 5px;
        border: none;
        background-color: transparent;
        color: white;
        font-family: "Inter", sans-serif;
        text-align: left;
        cursor: pointer;
        transition: ease color .12s;
    }

    .option.selected {
        color: #4677FF;
    }

    .rows {
        display: flex;
        flex-direction: column;
        row-gap: 4px;
    }

    .empty {
        padding: 8px;
        color: rgba(255, 255, 255, .5);
        text-align: center;
    }

    .footer {
        display: grid;
        margin-top: 10px;
    }
</style>
