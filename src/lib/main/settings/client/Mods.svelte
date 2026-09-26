<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { open as dialogOpen } from "@tauri-apps/plugin-dialog";
    import SettingWrapper from "../../../settings/SettingWrapper.svelte";
    import ModSetting from "../../../settings/ModSetting.svelte";
    import IconButtonSetting from "../../../settings/IconButtonSetting.svelte";
    import ButtonSetting from "../../../settings/ButtonSetting.svelte";
    import { capitalize } from "./copy.js";
    import { track, untrack } from "./modrinth.js";

    export let client;
    export let options;
    export let versionState;

    const dispatch = createEventDispatcher();

    $: build = versionState.currentBuild;

    let updating = new Set();

    async function addFile() {
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
                        options,
                        branch: build.branch,
                        mcVersion: build.mcVersion,
                        path: file
                    });
                }

                dispatch("updateMods");
            }
        } catch (e) {
            console.error("Failed to install mod:", e);
            alert(`Failed to install mod: ${e}`);
        }
    }

    function deleteFile(fileName) {
        return invoke("delete_custom_mod", {
            options,
            branch: build.branch,
            mcVersion: build.mcVersion,
            modName: fileName
        });
    }

    async function remove(mod) {
        try {
            if (mod.source.type === "local") {
                await deleteFile(mod.source.fileName);
            } else {
                await untrack(options, build, mod.modrinth.projectId);
            }
        } catch (e) {
            console.error("Failed to delete mod:", e);
            alert(`Failed to delete mod: ${e}`);
        }
        dispatch("updateMods");
    }

    // A file Modrinth knows becomes a Modrinth mod on the newer version.
    async function update(mod) {
        const key = modKey(mod);
        updating = new Set(updating).add(key);
        try {
            const entry = await invoke("modrinth_install", { client, options, projectId: mod.modrinth.projectId });
            await track(options, build, entry);
            if (mod.source.type === "local") {
                await deleteFile(mod.source.fileName);
            }
        } catch (e) {
            console.error("Failed to update mod:", e);
            alert(`${e}`);
        }
        updating.delete(key);
        updating = updating;
        dispatch("updateMods");
    }

    function modKey(mod) {
        return `${mod.source.type}:${mod.name}`;
    }
</script>

{#if build}
    <SettingWrapper title="Mods - {capitalize(build.subsystem)} {build.mcVersion}" unbounded>
        <div slot="title-element" class="actions">
            <IconButtonSetting text="Add file" icon="icon-plus" on:click={addFile} />
            <IconButtonSetting text="Browse" icon="icon-plus" on:click={() => dispatch("browse")} />
        </div>
        {#each versionState.recommendedMods as mod}
            <ModSetting
                    title={mod.name}
                    bind:value={mod.enabled}
                    disabled={mod.required}
                    removable={false}
                    on:change={() => dispatch("updateModStates")}
            />
        {/each}
        {#each versionState.customMods as mod (modKey(mod))}
            <ModSetting
                    title={mod.title}
                    bind:value={mod.enabled}
                    lined={!!mod.modrinth}
                    on:change={() => dispatch("updateModStates")}
                    on:delete={() => remove(mod)}
            >
                <svelte:fragment slot="line">
                    {#if mod.modrinth}
                        {mod.modrinth.version}{#if mod.modrinth.update}{" · "}<span class="strong">{mod.modrinth.update} available</span>{/if}
                    {/if}
                </svelte:fragment>
                <svelte:fragment slot="side">
                    {#if mod.modrinth?.update}
                        <ButtonSetting
                                small
                                text={updating.has(modKey(mod)) ? "Updating" : "Update"}
                                disabled={updating.has(modKey(mod))}
                                on:click={() => update(mod)}
                        />
                    {/if}
                </svelte:fragment>
            </ModSetting>
        {/each}
    </SettingWrapper>
{/if}

<style>
    .actions {
        display: flex;
        column-gap: 15px;
    }

    .strong {
        color: white;
    }
</style>
