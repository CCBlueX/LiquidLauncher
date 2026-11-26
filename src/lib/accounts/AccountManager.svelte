<script>
    import {createEventDispatcher} from "svelte";
    import {invoke} from "@tauri-apps/api/core";
    import {confirm} from "@tauri-apps/plugin-dialog";
    import SettingsContainer from "../settings/SettingsContainer.svelte";
    import TextSetting from "../settings/TextSetting.svelte";
    import ButtonSetting from "../settings/ButtonSetting.svelte";
    import IconButtonSetting from "../settings/IconButtonSetting.svelte";
    import Message from "../settings/Message.svelte";
    import AccountRow from "./AccountRow.svelte";
    import SignIn from "./SignIn.svelte";
    import {accounts, loadAccounts} from "./accounts.js";

    export let options;
    /** A running client writes the list when it exits, so it cannot be edited until then. */
    export let locked;

    const dispatch = createEventDispatcher();

    let adding = false;
    let search = "";
    let error = null;

    $: query = search.trim().toLowerCase();
    $: shown = $accounts.filter(entry => entry.name.toLowerCase().includes(query));

    async function edit(command, entry, args) {
        error = null;
        try {
            await invoke(command, {options, id: entry.id, ...args});
        } catch (e) {
            error = `${e}`;
        }
        await loadAccounts(options);
    }

    async function remove(entry) {
        const confirmed = await confirm(
            `Remove ${entry.name}? It disappears from LiquidBounce's account manager too.`
        );
        if (!confirmed) return;

        await edit("remove_account", entry);

        // An offline or session account cannot stay signed in without a Microsoft account.
        if (!await invoke("offline_allowed", {options})) {
            options.start.account = null;
            await options.store();
        }
    }

    async function added() {
        adding = false;
        await loadAccounts(options);
    }
</script>

<SettingsContainer title={adding ? "Add account" : "Accounts"} pinned on:hideSettings={() => dispatch("hide")}>
    <div class="top" slot="tabs">
        {#if adding}
            <IconButtonSetting text="Back" icon="icon-prev" on:click={() => adding = false} />
        {:else}
            <TextSetting placeholder="Search accounts" bind:value={search} on:keydown={e => e.key === "Escape" && (search = "")} />
        {/if}
    </div>

    {#if adding}
        <SignIn bind:options on:added={added} />
    {:else if shown.length}
        <div class="rows">
            {#each shown as entry (entry.id)}
                <AccountRow {entry} on:click={() => dispatch("switchAccount", entry)}>
                    {#if !locked}
                        {#if !entry.active}
                            <button class="action" type="button" title="Remove" aria-label="Remove {entry.name}" on:click={() => remove(entry)}>
                                <img src="img/icon/icon-trash.svg" alt="">
                            </button>
                        {/if}
                        <button
                                class="action"
                                class:shown={entry.favorite}
                                type="button"
                                title={entry.favorite ? "Remove from favorites" : "Add to favorites"}
                                aria-label="Favorite"
                                aria-pressed={entry.favorite}
                                on:click={() => edit("set_favorite", entry, {favorite: !entry.favorite})}
                        >
                            <img src="img/icon/icon-favorite{entry.favorite ? '-filled' : ''}.svg" alt="">
                        </button>
                    {/if}
                </AccountRow>
            {/each}
        </div>
    {:else}
        <Message title="No account matches “{search}”." clearable on:clear={() => search = ""} />
    {/if}

    <div class="footer" slot="footer">
        {#if !adding}
            {#if error}
                <p class="note error" role="alert">{error}</p>
            {:else if locked}
                <p class="note">Close LiquidBounce to add, remove or favorite accounts.</p>
            {/if}
            <ButtonSetting text="Add account" disabled={locked} on:click={() => adding = true} />
        {/if}
    </div>
</SettingsContainer>

<style>
    .top {
        display: flex;
        flex-direction: column;
        margin-bottom: 10px;
    }

    .top :global(.button) {
        align-self: start;
    }

    .rows {
        display: grid;
        row-gap: 4px;
    }

    .action {
        position: relative;
        display: flex;
        padding: 0;
        border: none;
        background: none;
        cursor: pointer;
        opacity: 0;
        transition: ease opacity .2s;
    }

    .action img {
        height: 18px;
    }

    .action.shown,
    :global(.row:hover) > .action,
    :global(.row:focus-within) > .action {
        opacity: 1;
    }

    .action:focus-visible {
        outline: solid 2px white;
        outline-offset: 2px;
    }

    .footer {
        display: grid;
        row-gap: 10px;
    }

    .footer:not(:empty) {
        margin-top: 10px;
    }

    .note {
        color: rgba(255, 255, 255, .5);
        font-size: 12px;
        line-height: 15px;
        text-align: center;
    }

    .note.error {
        color: #FE4C2E;
    }
</style>
