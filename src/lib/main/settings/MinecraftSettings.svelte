<script>
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import DirectorySelectorSetting from "../../settings/DirectorySelectorSetting.svelte";
    import {onMount} from "svelte";
    import {invoke} from "@tauri-apps/api/core";
    import Description from "../../settings/Description.svelte";

    export let options;

    let installation = null;

    async function getInstallation() {
        try {
            installation = await invoke("get_minecraft_installation", {
                customPath: options.start.installation.customPath || null 
            });
        } catch (e) {
            console.error("Failed to get vanilla status:", e);
        }
    }

    onMount(getInstallation);
    $: if (options.start.installation.customPath !== undefined) {
        getInstallation();
    }
</script>

<Description description="This will allow you to use worlds, resource packs, and shader packs from another installation of Minecraft." />

<DirectorySelectorSetting
        title="Minecraft Directory"
        placeholder={installation?.path || "Auto-detect"}
        bind:value={options.start.installation.customPath}
        windowTitle="Select Minecraft directory"
/>

{#if installation}
    <ToggleSetting
        title="Link worlds ({installation.saves_count})"
        disabled={false}
        bind:value={options.start.installation.useVanillaSaves}
    />

    <ToggleSetting
        title="Link resource packs ({installation.resource_packs_count})"
        disabled={false}
        bind:value={options.start.installation.useVanillaResourcePacks}
    />

    <ToggleSetting
        title="Link shader packs ({installation.shader_packs_count})"
        disabled={false}
        bind:value={options.start.installation.useVanillaShaderPacks}
    />
{:else}
    <Description description="No Minecraft vanilla installation found."/>
{/if}
