<script>
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import DirectorySelectorSetting from "../../settings/DirectorySelectorSetting.svelte";
    import {onMount} from "svelte";
    import {invoke} from "@tauri-apps/api/core";
    import Description from "../../settings/Description.svelte";

    export let options;

    let status = null;

    async function refreshStatus() {
        try {
            status = await invoke("get_vanilla_status", {
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

<Description description="This will allow you to use worlds, resource packs, and shader packs from another installation of Minecraft." />

<DirectorySelectorSetting
        title="Minecraft Directory"
        placeholder={status?.path || "Auto-detect"}
        bind:value={options.start.vanillaIntegration.customPath}
        windowTitle="Select Minecraft directory"
/>

{#if status?.found}
    <ToggleSetting
        title="Link worlds ({status.saves_count})"
        disabled={false}
        bind:value={options.start.vanillaIntegration.useVanillaSaves}
    />

    <ToggleSetting
        title="Link resource packs ({status.resource_packs_count})"
        disabled={false}
        bind:value={options.start.vanillaIntegration.useVanillaResourcePacks}
    />

    <ToggleSetting
        title="Link shader packs ({status.shader_packs_count})"
        disabled={false}
        bind:value={options.start.vanillaIntegration.useVanillaShaderPacks}
    />
{:else}
    <Description description="No Minecraft vanilla installation found."/>
{/if}
