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
            alert("Data cleared.");
        } catch (error) {
            console.error("Failed to clear data:", error);
            alert(`Failed to clear data: ${error}`);
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
    title="JVM Distribution"
    items={[
        { value: "automatic", text: "Automatic" },
        ...(distributions.length ? [{ value: "manual", text: "Manual" }] : []),
        { value: "custom", text: "Custom" },
    ]}
    on:change={resetJavaDistribution}
    bind:value={options.start.javaDistribution.type}
/>

{#if options.start.javaDistribution.type === "manual"}
    <SelectSetting
        title="Distribution"
        items={distributions.map(v => ({ value: v.vendor, text: v.name }))}
        bind:value={options.start.javaDistribution.value}
    />
{/if}

{#if options.start.javaDistribution.type === "custom"}
    <FileSelectorSetting
        title="Custom JVM Path"
        placeholder="Select Java wrapper location"
        bind:value={options.start.javaDistribution.value}
        filters={[{ name: "java", extensions: [] }]}
        windowTitle="Select custom Java wrapper"
    />
{/if}

<DirectorySelectorSetting
    title="Data Location"
    placeholder={defaultDataFolder}
    bind:value={options.start.customDataPath}
    windowTitle="Select custom data directory"
/>

<RangeSetting
    title="Memory"
    min={2048}
    max={systemMemory}
    bind:value={options.start.memory}
    valueSuffix=" MB"
    step={128}
/>

<RangeSetting
    title="Concurrent Downloads"
    min={1}
    max={50}
    bind:value={options.launcher.concurrentDownloads}
    valueSuffix=" connections"
    step={1}
/>

<ToggleSetting
    title="Keep launcher running"
    disabled={false}
    bind:value={options.launcher.keepLauncherOpen}
/>

<ButtonSetting text="Clear Data" on:click={clearData} color="#B83529" />

<LauncherVersion version={launcherVersion} />
