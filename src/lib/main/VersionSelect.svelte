<script>
    import {createEventDispatcher} from "svelte";
    import SettingsContainer from "../settings/SettingsContainer.svelte";
    import SelectSetting from "../settings/SelectSetting.svelte";
    import ToggleSetting from "../settings/ToggleSetting.svelte";
    import SettingWrapper from "../settings/SettingWrapper.svelte";
    import CustomModSetting from "../settings/CustomModSetting.svelte";
    import IconButtonSetting from "../settings/IconButtonSetting.svelte";
    import ModrinthSearch from "./ModrinthSearch.svelte";
    import ModrinthUpdates from "./ModrinthUpdates.svelte";
    import Tabs from "../settings/tab/Tabs.svelte";
    import {installedMods} from "../stores/modsStore.js";
    import {invoke} from "@tauri-apps/api/core";
    import {open as dialogOpen} from "@tauri-apps/plugin-dialog";

    export let options;
    export let versionState = {
        builds: [],
        branches: [],
        recommendedMods: [],
        customMods: [],
        currentBuild: null
    };

    let activeTab = "Version";
    const dispatch = createEventDispatcher();

    // Sync store with versionState.customMods whenever it changes
    $: installedMods.setMods(versionState.customMods);

    async function deleteMod(event) {
        try {
            await invoke("delete_custom_mod", {
                branch: versionState.currentBuild.branch,
                mcVersion: versionState.currentBuild.mcVersion,
                modName: `${event.detail.name}.jar`
            });
            // Update store immediately for instant UI feedback
            installedMods.removeMod(event.detail.name);
            dispatch('updateMods');
        } catch (error) {
            console.error("Failed to delete mod:", error);
            alert(`Failed to delete mod: ${error}`);
        }
    }

    async function installMod() {
        try {
            const selected = await dialogOpen({
                directory: false,
                multiple: true,
                filters: [{ name: "", extensions: ["jar"] }],
                title: "Select a custom mod to install"
            });

            if (selected) {
                for (const file of selected) {
                    await invoke("install_custom_mod", {
                        branch: versionState.currentBuild.branch,
                        mcVersion: versionState.currentBuild.mcVersion,
                        path: file
                    });
                }

                dispatch('updateMods');
            }
        } catch (error) {
            console.error("Failed to install mod:", error);
            alert(`Failed to install mod: ${error}`);
        }
    }
</script>

<SettingsContainer
        title="Select version"
        on:hideSettings={() => dispatch('hide')}
>
    <Tabs
            tabs={["Version", "Mods"]}
            bind:activeTab={activeTab}
            slot="tabs"
    />

    {#if activeTab === "Version"}
        <SelectSetting
                title="Branch"
                items={versionState.branches.map(e => ({
                    value: e,
                    text: `${e.charAt(0).toUpperCase()}${e.slice(1)} ${e === "legacy" ? "(unsupported)" : ""}`
                }))}
                bind:value={options.version.branchName}
                on:change={() => dispatch('updateData')}
        />
        <SelectSetting
                title="Build"
                items={[
                    { value: -1, text: "Latest" },
                    ...versionState.builds.map(e => ({
                        value: e.buildId,
                        text: `${e.lbVersion} git-${e.commitId.substring(0, 7)} - ${e.date}`
                    }))
                ]}
                bind:value={options.version.buildId}
                on:change={() => dispatch('updateData')}
        />
        <ToggleSetting
                title="Show nightly builds"
                bind:value={options.launcher.showNightlyBuilds}
                disabled={false}
                on:change={() => dispatch('updateData')}
        />
        <SettingWrapper title="Recommended mods">
            {#each versionState.recommendedMods as mod}
                <ToggleSetting
                        title={mod.name}
                        bind:value={mod.enabled}
                        disabled={mod.required}
                        on:change={() => dispatch('updateModStates')}
                />
            {/each}
        </SettingWrapper>
        <SettingWrapper title={`Additional mods - ${versionState.currentBuild?.subsystem ? `${versionState.currentBuild.subsystem.charAt(0).toUpperCase()}${versionState.currentBuild.subsystem.slice(1)}` : ''} ${versionState.currentBuild?.mcVersion}`}>
            <div slot="title-element">
                <IconButtonSetting
                        text="Install"
                        icon="icon-plus"
                        on:click={installMod}
                />
            </div>
            {#each versionState.customMods as mod}
                <CustomModSetting
                        title={mod.name}
                        bind:value={mod.enabled}
                        on:change={() => dispatch('updateModStates')}
                        on:delete={deleteMod}
                />
            {/each}
        </SettingWrapper>
    {:else if activeTab === "Mods"}
        <ModrinthSearch
            mcVersion={versionState.currentBuild?.mcVersion || ""}
            loader={versionState.currentBuild?.subsystem || "fabric"}
            branch={versionState.currentBuild?.branch || ""}
            on:installed={() => dispatch('updateMods')}
        />
        <ModrinthUpdates
            mcVersion={versionState.currentBuild?.mcVersion || ""}
            loader={versionState.currentBuild?.subsystem || "fabric"}
            branch={versionState.currentBuild?.branch || ""}
            on:updated={() => dispatch('updateMods')}
        />
    {/if}
</SettingsContainer>
