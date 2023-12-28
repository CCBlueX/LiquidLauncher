<script>
    import { createEventDispatcher } from "svelte";
    import ButtonClose from "../common/ButtonClose.svelte";
    import Logo from "../common/Logo.svelte";
    import TitleBar from "../common/TitleBar.svelte";
    import VerticalFlexWrapper from "../common/VerticalFlexWrapper.svelte";
    import ButtonSetting from "../settings/ButtonSetting.svelte";
    import RangeSetting from "../settings/RangeSetting.svelte";
    import SelectSetting from "../settings/SelectSetting.svelte";
    import SettingsContainer from "../settings/SettingsContainer.svelte";
    import SettingWrapper from "../settings/SettingWrapper.svelte";
    import ToggleSetting from "../settings/ToggleSetting.svelte";
    import Account from "./Account.svelte";
    import ContentWrapper from "./ContentWrapper.svelte";
    import LaunchArea from "./LaunchArea.svelte";
    import ClientLog from "./log/ClientLog.svelte";
    import NewsArea from "./news/NewsArea.svelte";
    import ProgressStatus from "./statusbar/ProgressStatus.svelte";
    import StatusBar from "./statusbar/StatusBar.svelte";
    import TextStatus from "./statusbar/TextStatus.svelte";
    import { invoke } from "@tauri-apps/api/tauri";
    import { listen } from "@tauri-apps/api/event";
    import DirectorySelectorSetting from "../settings/DirectorySelectorSetting.svelte";
    import FileSelectorSetting from "../settings/FileSelectorSetting.svelte";
    import LauncherVersion from "../settings/LauncherVersion.svelte";
    import IconButtonSetting from "../settings/IconButtonSetting.svelte";
    import CustomModSetting from "../settings/CustomModSetting.svelte";
    import { open as dialogOpen } from "@tauri-apps/api/dialog";

    export let options;

    const dispatch = createEventDispatcher();

    let defaultDataFolder = "";

    let settingsShown = false;
    let versionSelectShown = false;
    let clientLogShown = false;

    let clientRunning = false;

    let versionInfo = {
        bannerUrl: "img/banner.png", // TODO: placeholder image
        title: "Loading...",
        date: "Loading...",
        description: "Loading...",
    };

    let progressBar = {
        max: 0,
        value: 0,
        text: ""
    };
    
    let recommendedMods = [];
    let customMods = [];
    let branches = [];
    let builds = [];
    let currentBuild = {};

    let launcherVersion = "";

    let additionalModsTitle = "";
    $: {
        additionalModsTitle = `Additional mods for ${currentBuild.branch} ${currentBuild.mcVersion}`;
    }

    let log = [];

    invoke("get_launcher_version")
        .then(res => (launcherVersion = res));

    listen("process-output", event => {
        log = [...log, event.payload];
    });

    listen("progress-update", event => {
        let progressUpdate = event.payload;

        switch (progressUpdate.type) {
            case "max": {
                progressBar.max = progressUpdate.value;
                break;
            }
            case "progress": {
                progressBar.value = progressUpdate.value;
                break;
            }
            case "label": {
                progressBar.text = progressUpdate.value;
                break;
            }
        }
    });

    function getBuild() {
        if (options.preferredBuild === -1) { // -1 = latest
            // The find() method returns a value of the first element in the array that satisfies the provided testing function. Otherwise undefined is returned.
            return builds.find(e => e.release || options.showNightlyBuilds);
        }

        return builds.find((build) => build.buildId === options.preferredBuild);
    }

    function hideSettings() {
        settingsShown = false;
        options.store();
    }

    function hideVersionSelection() {
        versionSelectShown = false;
        options.store();
    }

    function updateModStates() {
        const branchOptions = {
            modStates: {},
            customModStates: {}
        };

        for (const mod of recommendedMods) {
            branchOptions.modStates[mod.name] = mod.enabled;
        }

        for (const mod of customMods) {
            branchOptions.customModStates[mod.name] = mod.enabled;
        }

        options.branchOptions[options.preferredBranch] = branchOptions;
        options.store();
    }

    /// Request builds from API server
    async function requestBuilds() {
        const requestedBuilds = await invoke("request_builds", { branch: options.preferredBranch });
        requestedBuilds.forEach(build => {
            const date = new Date(build.date);
            build.date = date.toLocaleString();
            build.dateDay = date.toLocaleDateString();
        });

        builds = requestedBuilds;

        await updateData();
    }

    /// Update build data
    async function updateData() {
        currentBuild = getBuild();

        // Update changelog
        const changelog = await invoke("fetch_changelog", { buildId: currentBuild.buildId });
        versionInfo = {
            bannerUrl: "img/banner.png",
            title: `LiquidBounce ${currentBuild.lbVersion} for Minecraft ${currentBuild.mcVersion}`,
            date: currentBuild.dateDay,
            description: changelog.changelog
        };

        requestMods();
    }

    /// Request mods from API server
    async function requestMods() {
        const { branch, mcVersion, subsystem } = currentBuild; 
        const branchOptions = options.branchOptions[branch];

        recommendedMods = await invoke("request_mods", { branch, mcVersion, subsystem });
        customMods = await invoke("get_custom_mods", { branch, mcVersion });

        if (branchOptions) {
            recommendedMods = recommendedMods.map(mod => {
                return { ...mod, enabled: branchOptions.modStates[mod.name] ?? mod.enabled };
            });

            customMods = customMods.map(mod => {
                return { ...mod, enabled: branchOptions.customModStates[mod.name] ?? mod.enabled };
            });
        }
    }

    async function runClient() {
        console.log("Client started");
        log = [];
        clientRunning = true;

        let build = getBuild();
        console.debug("Running build", build);

        console.log([...recommendedMods, ...customMods])
        await invoke("run_client", { buildId: build.buildId, accountData: options.currentAccount, options: options, mods: [...recommendedMods, ...customMods] });
    }
    
    async function terminateClient() {
        await invoke("terminate");
    }

    // Request branches from API server
    invoke("request_branches")
        .then(result => {
            // string array of branches
            branches = result.branches;

            // Default to first branch and latest build
            if (options.preferredBranch === null) {
                options.preferredBranch = result.defaultBranch;
                options.preferredBuild = -1;
            }

            // request builds of branch
            requestBuilds();
        })
        .catch(e => console.error(e));

    listen("client-exited", () => {
        clientRunning = false;
    });

    listen("client-error", (e) => {
        const message = e.payload;
        clientLogShown = true;

        alert(message);
    });

    function clearData() {
        invoke("clear_data", { options }).then(() => {
            alert("Data cleared.");
        }).catch(e => {
            alert("Failed to clear data: " + e);
            console.error(e)
        });
    }

    invoke("default_data_folder_path").then(result => {
        defaultDataFolder = result;
    }).catch(e => {
        alert("Failed to get data folder: " + e);
        console.error(e);
    });

    async function handleCustomModDelete(e) {
        const { branch, mcVersion } = currentBuild;

        await invoke("delete_custom_mod", { branch, mcVersion, modName: `${e.detail.name}.jar` });

        requestMods();
    }

    async function handleInstallMod(e) {
        const { branch, mcVersion } = currentBuild;

        const selected = await dialogOpen({
            directory: false,
            multiple: true,
            filters: [{ name: "", extensions: ["jar"] }],
            title: "Select a custom mod to install"
        });


        if (selected) {
            for (const path of selected) {
                await invoke("install_custom_mod", { branch, mcVersion, path });
            }

            requestMods();
        }
    }
</script>

{#if clientLogShown}
    <ClientLog messages={log} on:hideClientLog={() => clientLogShown = false} />
{/if}

{#if settingsShown}
    <SettingsContainer title="Settings" on:hideSettings={hideSettings}>
        <FileSelectorSetting title="JVM Location" placeholder="Internal" bind:value={options.customJavaPath} filters={[{ name: "javaw", extensions: [] }]} windowTitle="Select custom Java wrapper" />
        <DirectorySelectorSetting title="Data Location" placeholder={defaultDataFolder} bind:value={options.customDataPath} windowTitle="Select custom data directory" />
        <RangeSetting title="Memory" min={20} max={100} bind:value={options.memoryPercentage} valueSuffix="%" step={1} />
        <RangeSetting title="Concurrent Downloads" min={1} max={50} bind:value={options.concurrentDownloads} valueSuffix="connections" step={1} />
        <ToggleSetting title="Keep launcher running" disabled={false} bind:value={options.keepLauncherOpen} />
        <ButtonSetting text="Logout" on:click={() => dispatch("logout")} color="#4677FF" />
        <ButtonSetting text="Clear data" on:click={clearData} color="#B83529" />
        <LauncherVersion version={launcherVersion} />
    </SettingsContainer>
{/if}

{#if versionSelectShown}
    <SettingsContainer title="Select version" on:hideSettings={hideVersionSelection}>
        <SelectSetting title="Branch" items={branches.map(e => ({ value: e, text: e }))} bind:value={options.preferredBranch} on:change={requestBuilds} />
        <SelectSetting title="Build" items={[{ value: -1, text: "Latest" }, ...builds.filter(e => e.release || options.showNightlyBuilds).map(e => ({ value: e.buildId, text: e.lbVersion + " git-" + e.commitId.substring(0, 7) + " - " + e.date }))]} bind:value={options.preferredBuild} on:change={updateData} />
        <ToggleSetting title="Show nightly builds" bind:value={options.showNightlyBuilds} disabled={false} on:change={updateData} />
        <SettingWrapper title="Recommended mods">
            {#each recommendedMods as m}
                <ToggleSetting title={m.name} bind:value={m.enabled} disabled={m.required} on:change={updateModStates} />
            {/each}
        </SettingWrapper>
        <SettingWrapper title={additionalModsTitle}>
            <div slot="title-element">
                <IconButtonSetting text="Install" icon="icon-plus" on:click={handleInstallMod} />
            </div>

            {#each customMods as m}
                <CustomModSetting title={m.name} bind:value={m.enabled} on:change={updateModStates} on:delete={handleCustomModDelete} />
            {/each}
        </SettingWrapper>
    </SettingsContainer>
{/if}

<VerticalFlexWrapper blur={settingsShown || versionSelectShown || clientLogShown}>
    <TitleBar>
        <Logo />
        <StatusBar>
            {#if !clientRunning}
                <TextStatus text="Welcome back, {options.currentAccount.name}." />
            {:else}
                <ProgressStatus {...progressBar} />
            {/if}
        </StatusBar>
        <Account username={options.currentAccount.name} uuid={options.currentAccount.uuid} accountType={options.currentAccount.type} on:showSettings={() => settingsShown = true} />
        <ButtonClose />
    </TitleBar>

    <ContentWrapper>
        <LaunchArea {versionInfo} lbVersion={currentBuild.lbVersion} mcVersion={currentBuild.mcVersion} on:showVersionSelect={() => versionSelectShown = true} on:showClientLog={() => clientLogShown = true} on:launch={runClient} on:terminate={terminateClient} running={clientRunning} />
        <NewsArea />
    </ContentWrapper>
</VerticalFlexWrapper>
