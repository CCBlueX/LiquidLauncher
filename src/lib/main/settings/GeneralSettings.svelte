<script>
    import SelectSetting from "../../settings/SelectSetting.svelte";
    import FileSelectorSetting from "../../settings/FileSelectorSetting.svelte";
    import DirectorySelectorSetting from "../../settings/DirectorySelectorSetting.svelte";
    import RangeSetting from "../../settings/RangeSetting.svelte";
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import ButtonSetting from "../../settings/ButtonSetting.svelte";
    import LauncherVersion from "../../settings/LauncherVersion.svelte";
    import {onMount} from "svelte";
    import {invoke} from "@tauri-apps/api/core";
    import {_} from "svelte-i18n";
    import {languages} from "../../../i18n.js";

    export let client;
    export let options;

    let launcherVersion = "";
    let defaultDataFolder = "";
    let systemMemory = options.start.memory;
    let vendors = [];

    const isListed = id => vendors.some(v => v.vendor === id);

    // A saved distribution stays selectable after the API stops listing it.
    $: saved = options.start.javaDistribution;
    $: distributions = saved.type === "manual" && !isListed(saved.value)
        ? [...vendors, { vendor: saved.value, name: saved.value }]
        : vendors;

    async function clearData() {
        try {
            await invoke("clear_data", { options });
            alert($_("settings.general.dataCleared"));
        } catch (error) {
            console.error("Failed to clear data:", error);
            alert($_("settings.general.clearDataFailed", { values: { error: `${error}` } }));
        }
    }

    async function logout() {
        try {
            await invoke("logout", { accountData: options.start.account });
            options.start.account = null;
            await options.store();
        } catch (error) {
            console.error("Logout failed:", error);
            alert($_("settings.general.logoutFailed"));
        }
    }

    function resetJavaDistribution() {
        if (options.start.javaDistribution.type === "custom") {
            options.start.javaDistribution.value = "";
        } else if (options.start.javaDistribution.type === "manual" && !isListed(options.start.javaDistribution.value)) {
            options.start.javaDistribution.value = vendors[0].vendor;
        }
    }

    onMount(async () => {
        invoke("get_java_distributions", { client })
            .then(listed => vendors = listed)
            .catch(error => console.error("Failed to load Java distributions:", error));

        const [version, folder, memory] = await Promise.all([
            invoke("get_launcher_version"),
            invoke("default_data_folder_path"),
            invoke("sys_memory"),
        ]);

        systemMemory = memory;
        launcherVersion = version;
        defaultDataFolder = folder;
    });
</script>

<SelectSetting
    title={$_("settings.general.language")}
    items={[
        { value: null, text: $_("settings.general.systemLanguage") },
        ...$languages.map(({ code, name }) => ({ value: code, text: name })),
    ]}
    bind:value={options.launcher.language}
/>

<SelectSetting
    title={$_("settings.general.jvmDistribution")}
    items={[
        { value: "automatic", text: $_("settings.general.automatic") },
        ...(distributions.length ? [{ value: "manual", text: $_("settings.general.manual") }] : []),
        { value: "custom", text: $_("settings.general.custom") },
    ]}
    on:change={resetJavaDistribution}
    bind:value={options.start.javaDistribution.type}
/>

{#if options.start.javaDistribution.type === "manual"}
    <SelectSetting
        title={$_("settings.general.distribution")}
        items={distributions.map(v => ({ value: v.vendor, text: v.name }))}
        bind:value={options.start.javaDistribution.value}
    />
{/if}

{#if options.start.javaDistribution.type === "custom"}
    <FileSelectorSetting
        title={$_("settings.general.customJvmPath")}
        placeholder={$_("settings.general.customJvmPlaceholder")}
        bind:value={options.start.javaDistribution.value}
        filters={[{ name: "java", extensions: [] }]}
        windowTitle={$_("settings.general.customJvmWindowTitle")}
    />
{/if}

<DirectorySelectorSetting
    title={$_("settings.general.dataLocation")}
    placeholder={defaultDataFolder}
    bind:value={options.start.customDataPath}
    windowTitle={$_("settings.general.dataLocationWindowTitle")}
/>

<RangeSetting
    title={$_("settings.general.memory")}
    min={2048}
    max={systemMemory}
    bind:value={options.start.memory}
    valueSuffix=" MB"
    step={128}
/>

<RangeSetting
    title={$_("settings.general.concurrentDownloads")}
    min={1}
    max={50}
    bind:value={options.launcher.concurrentDownloads}
    valueSuffix={$_("settings.general.connectionsSuffix")}
    step={1}
/>

<ToggleSetting
    title={$_("settings.general.keepLauncherOpen")}
    disabled={false}
    bind:value={options.launcher.keepLauncherOpen}
/>

<ButtonSetting
    text={$_("settings.general.signOut")}
    on:click={logout}
    color="#4677FF"
/>

<ButtonSetting text={$_("settings.general.clearData")} on:click={clearData} color="#B83529" />

<LauncherVersion version={launcherVersion} />
