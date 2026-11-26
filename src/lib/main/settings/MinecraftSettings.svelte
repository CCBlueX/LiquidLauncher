<script>
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import DirectorySelectorSetting from "../../settings/DirectorySelectorSetting.svelte";
    import {onMount} from "svelte";
    import {invoke} from "@tauri-apps/api/core";
    import Description from "../../settings/Description.svelte";
    import {_} from "svelte-i18n";

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

<Description description={$_("settings.minecraft.description")} />

<DirectorySelectorSetting
        title={$_("settings.minecraft.directory")}
        placeholder={installation?.path || $_("settings.minecraft.autoDetect")}
        bind:value={options.start.installation.customPath}
        windowTitle={$_("settings.minecraft.directoryWindowTitle")}
/>

{#if installation}
    <ToggleSetting
        title={$_("settings.minecraft.linkWorlds", { values: { count: installation.saves_count } })}
        disabled={false}
        bind:value={options.start.installation.useVanillaSaves}
    />

    <ToggleSetting
        title={$_("settings.minecraft.linkResourcePacks", { values: { count: installation.resource_packs_count } })}
        disabled={false}
        bind:value={options.start.installation.useVanillaResourcePacks}
    />

    <ToggleSetting
        title={$_("settings.minecraft.linkShaderPacks", { values: { count: installation.shader_packs_count } })}
        disabled={false}
        bind:value={options.start.installation.useVanillaShaderPacks}
    />
{:else}
    <Description description={$_("settings.minecraft.notFound")}/>
{/if}
