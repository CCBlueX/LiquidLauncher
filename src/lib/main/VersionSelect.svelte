<script>
    import { createEventDispatcher } from "svelte";
    import SettingsContainer from "../settings/SettingsContainer.svelte";
    import SelectSetting from "../settings/SelectSetting.svelte";
    import ToggleSetting from "../settings/ToggleSetting.svelte";
    import SettingWrapper from "../settings/SettingWrapper.svelte";
    import CustomModSetting from "../settings/CustomModSetting.svelte";
    import IconButtonSetting from "../settings/IconButtonSetting.svelte";
    import {invoke} from "@tauri-apps/api/core";
    import { open as dialogOpen } from "@tauri-apps/plugin-dialog";

    export let options;
    export let versionState = {
        builds: [],
        branches: [],
        recommendedMods: [],
        customMods: [],
        currentBuild: null
    };

    const dispatch = createEventDispatcher();

    async function deleteMod(event) {
        try {
            await invoke("delete_custom_mod", {
                branch: versionState.currentBuild.branch,
                mcVersion: versionState.currentBuild.mcVersion,
                modName: `${event.detail.name}.jar`
            });
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
    <SettingWrapper title={`Additional mods for ${versionState.currentBuild?.branch} ${versionState.currentBuild?.mcVersion}`}>
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
</SettingsContainer>