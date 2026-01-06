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

<DirectorySelectorSetting
    title="Minecraft Directory"
    placeholder={vanillaStatus?.path || "Auto-detect"}
    bind:value={options.start.vanillaIntegration.customPath}
    windowTitle="Select Minecraft directory"
/>

{#if vanillaStatus?.found}
    <div class="vanilla-info">
        {vanillaStatus.saves_count} worlds • {vanillaStatus.resource_packs_count} resource packs • {vanillaStatus.shader_packs_count} shader packs
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
    .vanilla-info {
        color: #666;
        font-size: 11px;
        margin-bottom: 8px;
    }

    .vanilla-not-found {
        color: #888;
        font-size: 12px;
    }
</style>
