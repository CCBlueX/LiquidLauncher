<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { scale, fade } from "svelte/transition";

    export let mcVersion;
    export let loader;
    export let branch;

    let mods = [];
    let checking = false;
    let updating = {};
    let removing = {};
    let lastKey = "";

    const dispatch = createEventDispatcher();

    export async function checkUpdates() {
        if (!mcVersion || !branch) return;
        checking = true;
        try {
            // First sync any existing mods not yet tracked
            const synced = await invoke("modrinth_sync_existing", {
                branch,
                mcVersion
            });
            if (synced > 0) {
                console.log(`Synced ${synced} existing mods with Modrinth`);
                dispatch("synced");
            }

            mods = await invoke("modrinth_check_updates", {
                branch,
                mcVersion,
                loader
            });
        } catch (e) {
            console.error("Failed to check updates:", e);
            mods = [];
        }
        checking = false;
    }

    async function updateMod(mod) {
        updating = { ...updating, [mod.info.project_id]: true };
        
        try {
            await invoke("modrinth_update_mod", {
                projectId: mod.info.project_id,
                mcVersion,
                loader,
                branch
            });
            mod.has_update = false;
            mod.new_version = null;
            mods = mods;
            dispatch("updated");
        } catch (e) {
            console.error("Update failed:", e);
        }
        
        updating = { ...updating, [mod.info.project_id]: false };
    }

    async function uninstallMod(mod) {
        removing = { ...removing, [mod.info.project_id]: true };
        try {
            await invoke("delete_custom_mod", {
                branch,
                mcVersion,
                modName: mod.info.filename
            });
            await checkUpdates();
            dispatch("removed");
        } catch (e) {
            console.error("Uninstall failed:", e);
        }
        removing = { ...removing, [mod.info.project_id]: false };
    }

    async function updateAll() {
        const toUpdate = mods.filter(m => m.has_update);
        for (const mod of toUpdate) {
            await updateMod(mod);
        }
    }

    $: hasUpdates = mods.some(m => m.has_update);
    $: updateCount = mods.filter(m => m.has_update).length;

    $: if (mcVersion && branch && loader) {
        const nextKey = `${branch}:${mcVersion}:${loader}`;
        if (nextKey !== lastKey) {
            lastKey = nextKey;
            checkUpdates();
        }
    }
</script>

{#if mods.length > 0}
    <div class="updates-section">
        <div class="updates-header">
            <span class="title">
                Modrinth Mods
                {#if hasUpdates}
                    <span class="update-count" in:scale={{ duration: 200 }}>{updateCount}</span>
                {/if}
            </span>
            <div class="actions">
                {#if hasUpdates}
                <button 
                    class="update-all-btn" 
                    on:click={updateAll} 
                    in:fade={{ duration: 150 }}
                    aria-label="Update all mods"
                >
                    Update All
                </button>
                {/if}
                <button
                    class="refresh-btn"
                    on:click={checkUpdates}
                    disabled={checking}
                    aria-label="Refresh updates"
                >
                    {#if checking}
                        <span class="spinner"></span>
                    {:else}
                        <img class="icon" src="img/icon/icon-refresh.svg" alt="" />
                    {/if}
                </button>
            </div>
        </div>

        <div class="mods-list">
            {#each mods as mod (mod.info.project_id)}
                <div class="mod-row" class:has-update={mod.has_update}>
                    <span class="mod-name">{mod.info.title}</span>
                    <div class="mod-actions">
                        {#if mod.has_update}
                            <span class="new-version" in:fade={{ duration: 150 }}>
                                → {mod.new_version}
                            </span>
                            <button
                                class="icon-btn update-btn"
                                on:click={() => updateMod(mod)}
                                disabled={updating[mod.info.project_id]}
                                aria-label={`Update ${mod.info.title} to version ${mod.new_version}`}
                            >
                                {#if updating[mod.info.project_id]}
                                    <span class="spinner small"></span>
                                {:else}
                                    <img class="icon" src="img/icon/icon-download.svg" alt="" />
                                {/if}
                            </button>
                        {:else}
                            <span class="up-to-date">✓</span>
                        {/if}
                        <button
                            class="icon-btn delete-btn"
                            on:click={() => uninstallMod(mod)}
                            disabled={removing[mod.info.project_id]}
                            aria-label={`Uninstall ${mod.info.title}`}
                        >
                            {#if removing[mod.info.project_id]}
                                <span class="spinner small"></span>
                            {:else}
                                <img class="icon" src="img/icon/icon-trash.svg" alt="" />
                            {/if}
                        </button>
                    </div>
                </div>
            {/each}
        </div>
    </div>
{/if}

<style>
    .updates-section {
        margin-top: 8px;
        padding-top: 12px;
        border-top: 1px solid rgba(255, 255, 255, 0.1);
    }

    .updates-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 10px;
    }

    .title {
        color: #888;
        font-size: 11px;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .update-count {
        background: #4677ff;
        color: white;
        font-size: 10px;
        padding: 2px 6px;
        border-radius: 10px;
        min-width: 18px;
        text-align: center;
    }

    .actions {
        display: flex;
        gap: 8px;
        align-items: center;
    }

    .update-all-btn {
        background: #4677ff;
        border: none;
        border-radius: 4px;
        padding: 4px 10px;
        color: white;
        font-size: 11px;
        font-weight: 600;
        cursor: pointer;
        transition: background 0.2s;
    }

    .update-all-btn:hover {
        background: #5a88ff;
    }

    .refresh-btn {
        background: transparent;
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 4px;
        width: 28px;
        height: 28px;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: all 0.2s;
    }

    .refresh-btn:hover:not(:disabled) {
        background: rgba(255, 255, 255, 0.1);
    }

    .refresh-btn:disabled {
        opacity: 0.5;
    }

    .refresh-btn .icon {
        width: 14px;
        height: 14px;
        color: #888;
    }

    .mods-list {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .mod-row {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 8px 10px;
        background: rgba(255, 255, 255, 0.03);
        border-radius: 6px;
        border: 1px solid transparent;
        transition: all 0.2s;
    }

    .mod-row.has-update {
        background: rgba(70, 119, 255, 0.1);
        border-color: rgba(70, 119, 255, 0.2);
    }

    .mod-name {
        flex: 1;
        color: white;
        font-size: 12px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .mod-actions {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .new-version {
        color: #4677ff;
        font-size: 11px;
        font-weight: 500;
    }

    .up-to-date {
        color: #60B675;
        font-size: 12px;
    }

    .icon-btn {
        border: none;
        border-radius: 4px;
        width: 28px;
        height: 28px;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: all 0.2s;
    }

    .update-btn {
        background: #4677ff;
    }

    .update-btn:hover:not(:disabled) {
        background: #5a88ff;
        transform: scale(1.05);
    }

    .delete-btn {
        background: #3f6dff;
        box-shadow: 0 0 0 2px rgba(70, 119, 255, 0.85), 0 0 18px rgba(70, 119, 255, 0.9);
    }

    .delete-btn:hover:not(:disabled) {
        background: #e55252;
        transform: scale(1.14);
        box-shadow: 0 0 0 2px rgba(229, 82, 82, 0.35), 0 6px 14px rgba(229, 82, 82, 0.35);
        animation: trash-wiggle 0.22s ease-in-out 1;
    }

    .icon-btn:disabled {
        opacity: 0.7;
    }

    .icon-btn .icon {
        width: 14px;
        height: 14px;
        color: white;
    }

    @keyframes trash-wiggle {
        0% { transform: scale(1.06) rotate(0deg); }
        35% { transform: scale(1.06) rotate(-6deg); }
        70% { transform: scale(1.06) rotate(6deg); }
        100% { transform: scale(1.06) rotate(0deg); }
    }

    .spinner {
        width: 14px;
        height: 14px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 0.7s linear infinite;
    }

    .spinner.small {
        width: 12px;
        height: 12px;
    }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }
</style>
