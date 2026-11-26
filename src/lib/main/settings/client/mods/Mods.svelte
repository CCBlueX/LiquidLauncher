<script context="module">
    // The options keep the mods installed from Modrinth by branch and Minecraft version.
    function installed(options, build) {
        const branch = options.version.options[build.branch] ??= { modStates: {}, customModStates: {} };
        branch.modrinthMods ??= {};
        return branch.modrinthMods[build.mcVersion] ??= [];
    }

    export function track(options, build, entry) {
        const mods = installed(options, build);
        const index = mods.findIndex(mod => mod.projectId === entry.projectId);
        if (index === -1) {
            mods.push(entry);
        } else {
            mods[index] = entry;
        }
        return options.store();
    }
</script>

<script>
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { _ } from "svelte-i18n";
    import { open as dialogOpen } from "@tauri-apps/plugin-dialog";
    import SettingWrapper from "../../../../settings/SettingWrapper.svelte";
    import ModSetting from "../../../../settings/ModSetting.svelte";
    import IconButtonSetting from "../../../../settings/IconButtonSetting.svelte";
    import ButtonSetting from "../../../../settings/ButtonSetting.svelte";

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
                title: $_("mods.addFileWindowTitle")
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
            alert($_("mods.installFailed", { values: { error: `${e}` } }));
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
                const mods = installed(options, build);
                const index = mods.findIndex(entry => entry.projectId === mod.modrinth.projectId);
                if (index !== -1) {
                    mods.splice(index, 1);
                }
                await options.store();
            }
        } catch (e) {
            console.error("Failed to delete mod:", e);
            alert($_("mods.deleteFailed", { values: { error: `${e}` } }));
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

    // Recommended mods load from a Maven repository, as `group:name:version`.
    $: recommendedLine = mod => [mod.source.artifact?.split(":")[2], $_("mods.recommended")].filter(Boolean).join(" · ");

    function modKey(mod) {
        return `${mod.source.type}:${mod.name}`;
    }
</script>

{#if build}
    <SettingWrapper title={$_("mods.title", { values: { target: `${build.subsystem.charAt(0).toUpperCase()}${build.subsystem.slice(1)} ${build.mcVersion}` } })} unbounded>
        <div slot="title-element" class="actions">
            <IconButtonSetting text={$_("mods.addFile")} icon="icon-plus" on:click={addFile} />
            <IconButtonSetting text={$_("mods.browse")} icon="icon-plus" on:click={() => dispatch("browse")} />
        </div>
        {#each versionState.recommendedMods as mod}
            <ModSetting
                    title={mod.name}
                    bind:value={mod.enabled}
                    disabled={mod.required}
                    removable={false}
                    on:change={() => dispatch("updateModStates")}
            >
                <svelte:fragment slot="line">{recommendedLine(mod)}</svelte:fragment>
            </ModSetting>
        {/each}
        {#each versionState.customMods as mod (modKey(mod))}
            <ModSetting
                    title={mod.title}
                    bind:value={mod.enabled}
                    on:change={() => dispatch("updateModStates")}
                    on:delete={() => remove(mod)}
            >
                <svelte:fragment slot="line">
                    {mod.version ?? $_("mods.unknownVersion")}{#if mod.modrinth?.update}{" · "}<span class="strong">{$_("mods.updateAvailable", { values: { version: mod.modrinth.update } })}</span>{/if}
                </svelte:fragment>
                <svelte:fragment slot="side">
                    {#if mod.modrinth?.update}
                        <ButtonSetting
                                small
                                text={updating.has(modKey(mod)) ? $_("common.updating") : $_("common.update")}
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
