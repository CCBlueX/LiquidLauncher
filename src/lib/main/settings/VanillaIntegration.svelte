<script>
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import DirectorySelectorSetting from "../../settings/DirectorySelectorSetting.svelte";
    import {onMount} from "svelte";
    import {invoke} from "@tauri-apps/api/core";

    export let options;

    let vanillaStatus = null;

    async function refreshStatus() {
        try {
            vanillaStatus = await invoke("get_vanilla_status", { 
                customPath: options.start.vanillaIntegration.customPath || null 
            });
        } catch (e) {
            console.error("Failed to get vanilla status:", e);
        }
    }

    onMount(refreshStatus);

    $: if (options.start.vanillaIntegration.customPath !== undefined) {
        refreshStatus();
    }
</script>

<div class="section-title">Vanilla Integration</div>

<DirectorySelectorSetting
    title="Minecraft Directory"
    placeholder={vanillaStatus?.path || "Auto-detect"}
    bind:value={options.start.vanillaIntegration.customPath}
    windowTitle="Select Minecraft directory"
/>

{#if vanillaStatus?.found}
    <div class="vanilla-info">
        <span class="stats">
            {vanillaStatus.saves_count} worlds • {vanillaStatus.resource_packs_count} resource packs • {vanillaStatus.shader_packs_count} shader packs
        </span>
    </div>

    <ToggleSetting
        title="Use vanilla worlds"
        disabled={false}
        bind:value={options.start.vanillaIntegration.useVanillaSaves}
    />

    <ToggleSetting
        title="Use vanilla resource packs"
        disabled={false}
        bind:value={options.start.vanillaIntegration.useVanillaResourcePacks}
    />

    <ToggleSetting
        title="Use vanilla shader packs"
        disabled={false}
        bind:value={options.start.vanillaIntegration.useVanillaShaderPacks}
    />
{:else}
    <div class="vanilla-not-found">
        Minecraft installation not found
    </div>
{/if}

<style>
    .section-title {
        color: #4677ff;
        font-size: 12px;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }

    .vanilla-info {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .stats {
        color: #666;
        font-size: 11px;
    }

    .vanilla-not-found {
        color: #888;
        font-size: 12px;
    }
</style>
