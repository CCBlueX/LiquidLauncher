<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { flip } from "svelte/animate";
    import { fade, scale } from "svelte/transition";
    import { installedMods } from "../stores/modsStore.js";

    export let mcVersion;
    export let loader;
    export let branch;

    let query = "";
    let results = [];
    let searching = false;
    let installing = {}; // Tracks which mods are currently being installed
    let refreshKey = 0; // Force recalculation when store updates

    const dispatch = createEventDispatcher();

    // Reactive: converts store to lowercase names for efficient lookup
    // This recalculates whenever $installedMods changes
    $: installedNames = $installedMods.map(m => m.name.toLowerCase());
    
    // Force recalculation when installed mods change
    $: if ($installedMods) {
        refreshKey++;
    }

    /**
     * Check if a mod is already installed by comparing slugs and titles
     * with the installed mod filenames
     * Made reactive by depending on installedNames, results, and refreshKey
     * This ensures the UI updates immediately when mods are added/removed
     */
    $: isInstalledMap = (() => {
        // Explicitly depend on installedNames and refreshKey to ensure reactivity
        const names = installedNames;
        const _key = refreshKey; // Force recalculation
        return results.reduce((acc, project) => {
            const slug = project.slug.toLowerCase();
            const title = project.title.toLowerCase();
            acc[project.project_id] = names.some(modName => 
                modName.includes(slug) || slug.includes(modName) || 
                modName.includes(title) || title.includes(modName)
            );
            return acc;
        }, {});
    })();

    async function search() {
        if (!query.trim()) return;
        searching = true;
        try {
            results = await invoke("modrinth_search", {
                query: query.trim(),
                mcVersion,
                loader
            });
        } catch (e) {
            console.error("Search failed:", e);
            results = [];
        }
        searching = false;
    }

    async function install(project) {
        const id = project.project_id;
        installing[id] = true;
        installing = installing; // Trigger reactivity
        
        try {
            const filename = await invoke("modrinth_install", {
                projectId: id,
                mcVersion,
                loader,
                branch,
                title: project.title
            });
            // Add to store for instant UI update across all components
            installedMods.addMod({ name: filename.replace('.jar', ''), enabled: true });
            dispatch("installed");
        } catch (e) {
            console.error("Install failed:", e);
        }
        installing[id] = false;
        installing = installing;
    }

    function handleKeydown(e) {
        if (e.key === "Enter") search();
    }

    /**
     * Get the current state of a mod for UI rendering
     * Priority: installing > installed > idle
     * Uses reactive isInstalledMap for automatic updates
     */
    function getState(projectId) {
        if (installing[projectId]) return 'installing';
        return isInstalledMap[projectId] ? 'installed' : 'idle';
    }
</script>

<div class="modrinth-search">
    <div class="search-bar">
        <input
            type="text"
            placeholder="Search Modrinth mods..."
            aria-label="Search Modrinth mods"
            bind:value={query}
            on:keydown={handleKeydown}
        />
        <button 
            class="search-btn" 
            on:click={search} 
            disabled={searching}
            aria-label="Search"
        >
            {#if searching}
                <span class="spinner"></span>
            {:else}
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="11" cy="11" r="8"/>
                    <path d="m21 21-4.35-4.35"/>
                </svg>
            {/if}
        </button>
    </div>

    {#if results.length > 0}
        <div class="results">
            {#each results as project (project.project_id)}
                {@const state = installing[project.project_id] ? 'installing' : (isInstalledMap[project.project_id] ? 'installed' : 'idle')}
                <div 
                    class="mod-item" 
                    class:installed={state === 'installed'}
                    animate:flip={{ duration: 200 }}
                >
                    <img 
                        class="mod-icon" 
                        src={project.icon_url || "img/steve.png"} 
                        alt={`${project.title} icon`}
                        on:error={(e) => e.target.src = "img/steve.png"}
                    />
                    <div class="mod-info">
                        <span class="mod-title">
                            {project.title}
                            {#if state === 'installed'}
                                <span class="installed-badge" in:scale={{ duration: 200, start: 0.5 }}>
                                    Installed
                                </span>
                            {/if}
                        </span>
                        <span class="mod-meta">
                            {project.author} • {project.downloads.toLocaleString()} downloads
                        </span>
                    </div>
                    <button 
                        class="install-btn"
                        class:installing={state === 'installing'}
                        class:installed={state === 'installed'}
                        on:click={() => install(project)}
                        disabled={state !== 'idle'}
                        aria-label={state === 'installed' ? `${project.title} is installed` : state === 'installing' ? `Installing ${project.title}` : `Install ${project.title}`}
                    >
                        {#if state === 'installing'}
                            <span class="spinner small" in:fade={{ duration: 150 }}></span>
                        {:else if state === 'installed'}
                            <svg 
                                class="check" 
                                viewBox="0 0 24 24" 
                                fill="none" 
                                stroke="currentColor" 
                                stroke-width="3"
                                in:scale={{ duration: 300, start: 0 }}
                            >
                                <path d="M20 6L9 17l-5-5"/>
                            </svg>
                        {:else}
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M12 5v14M5 12h14"/>
                            </svg>
                        {/if}
                    </button>
                </div>
            {/each}
        </div>
    {/if}
</div>

<style>
    .modrinth-search {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .search-bar {
        display: flex;
        gap: 8px;
    }

    .search-bar input {
        flex: 1;
        background: rgba(255, 255, 255, 0.08);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 6px;
        padding: 10px 14px;
        color: white;
        font-size: 13px;
        transition: border-color 0.2s, background 0.2s;
    }

    .search-bar input:focus {
        outline: none;
        border-color: #4677ff;
        background: rgba(255, 255, 255, 0.12);
    }

    .search-bar input::placeholder {
        color: #666;
    }

    .search-btn {
        background: #4677ff;
        border: none;
        border-radius: 6px;
        width: 42px;
        height: 42px;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: background 0.2s, transform 0.1s;
    }

    .search-btn:hover:not(:disabled) {
        background: #5a88ff;
    }

    .search-btn:active:not(:disabled) {
        transform: scale(0.95);
    }

    .search-btn:disabled {
        opacity: 0.7;
        cursor: not-allowed;
    }

    .search-btn svg {
        width: 18px;
        height: 18px;
        color: white;
    }

    .results {
        display: flex;
        flex-direction: column;
        gap: 8px;
        max-height: 180px;
        overflow-y: auto;
        overflow-x: hidden;
        padding-right: 2px;
    }

    .results::-webkit-scrollbar {
        width: 3px;
    }

    .results::-webkit-scrollbar-track {
        background: transparent;
    }

    .results::-webkit-scrollbar-thumb {
        background: rgba(255, 255, 255, 0.15);
        border-radius: 3px;
    }

    .mod-item {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 10px 12px;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 8px;
        border: 1px solid transparent;
        transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .mod-item:hover {
        background: rgba(255, 255, 255, 0.08);
    }

    .mod-item.installed {
        background: rgba(96, 182, 117, 0.12);
        border-color: rgba(96, 182, 117, 0.25);
    }

    .mod-icon {
        width: 36px;
        height: 36px;
        border-radius: 6px;
        object-fit: cover;
        background: rgba(0, 0, 0, 0.3);
    }

    .mod-info {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 2px;
        min-width: 0;
    }

    .mod-title {
        color: white;
        font-size: 13px;
        font-weight: 500;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .installed-badge {
        font-size: 9px;
        font-weight: 600;
        text-transform: uppercase;
        background: #60B675;
        color: white;
        padding: 2px 6px;
        border-radius: 4px;
        letter-spacing: 0.3px;
        flex-shrink: 0;
    }

    .mod-meta {
        color: #888;
        font-size: 11px;
    }

    .install-btn {
        width: 36px;
        height: 36px;
        border: none;
        border-radius: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
        background: #60B675;
        flex-shrink: 0;
    }

    .install-btn:hover:not(:disabled) {
        background: #4a9c5f;
        transform: scale(1.08);
    }

    .install-btn:active:not(:disabled) {
        transform: scale(0.95);
    }

    .install-btn:disabled {
        cursor: default;
    }

    .install-btn svg {
        width: 18px;
        height: 18px;
        color: white;
    }

    .install-btn.installing {
        background: #4677ff;
        transform: scale(1);
    }

    .install-btn.installed {
        background: #60B675;
    }

    .spinner {
        width: 18px;
        height: 18px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 0.7s linear infinite;
    }

    .spinner.small {
        width: 16px;
        height: 16px;
    }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }
</style>
