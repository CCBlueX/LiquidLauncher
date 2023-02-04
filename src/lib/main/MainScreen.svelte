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
    import TextSetting from "../settings/TextSetting.svelte";
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
    import DirectorySelector from "../settings/DirectorySelector.svelte";

    export let options;

    const dispatch = createEventDispatcher();

    let versionInfo = {
        bannerUrl: "https://liquidbounce.net/LiquidLauncher/img/b73.jpg",
        title: "Loading...",
        date: "Loading...",
        description: "Loading...",
    };

    let settingsShown = false;
    let versionSelectShown = false;
    let clientLogShown = false;

    let log = [];

    let progressBarMax = 0;
    let progressBarProgress = 0;
    let progressBarLabel = "";

    listen("process-output", event => {
        log = [...log, event.payload];
    });

    listen("progress-update", event => {
        let progressUpdate = event.payload;
        console.log(event);

        switch (progressUpdate.type) {
                case "max": {
                    progressBarMax = progressUpdate.value;
                    break;
                }
                case "progress": {
                    progressBarProgress = progressUpdate.value;
                    break;
                }
                case "label": {
                    progressBarLabel = progressUpdate.value;
                    break;
                }
            }
    });

    let branches = [];

    let builds = [];

    function getBuild() {
        if (options.preferredBuild === -1) { // -1 = latest
            // The find() method returns a value of the first element in the array that satisfies the provided testing function. Otherwise undefined is returned.
            return builds.find(e => e.release || options.showNightlyBuilds);
        }

        return builds.find((build) => build.buildId === options.preferredBuild);
    }

    let lbVersion = {};
    let mcVersion = {};

    let mods = [];

    /// Request builds from API server
    function requestBuilds() {
        invoke("request_builds", { branch: options.preferredBranch })
            .then(result => {
                builds = result;

                // Format date for user readability
                builds.forEach(build => {
                    let date = new Date(build.date);
                    build.date = date.toLocaleString();
                    build.dateDay = date.toLocaleDateString();
                });

                updateData();
            })
            .catch(e => console.error(e));
    }

    /// Update build data
    function updateData() {
        let b = getBuild();
        console.debug("Updating build data", b);

        lbVersion = {
            date: b.date,
            title: b.lbVersion
        };
        mcVersion = {
            date: "", // todo: No date for MC version
            title: b.mcVersion
        };

        // Update changelog
        invoke("fetch_changelog", { buildId: b.buildId })
            .then(result => {
                console.log("Fetched changelog data", result);
                versionInfo = {
                    bannerUrl: "https://liquidbounce.net/LiquidLauncher/img/b73.jpg",
                    title: "LiquidBounce " + b.lbVersion + " for Minecraft " + b.mcVersion,
                    date: b.dateDay,
                    description: result.changelog,
                };
            })
            .catch(e => console.error(e));

        requestMods(b.mcVersion, b.subsystem);
    }

    /// Request mods from API server
    function requestMods(mcVersion, subsystem) {
        invoke("request_mods", { mcVersion, subsystem })
            .then(result => {
                mods = result;

                mods.forEach(mod => {
                    mod.enabled = options.modStates[mod.name] ?? mod.enabled;
                });
            })
            .catch(e => console.error(e));
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

    function hideSettings() {
        settingsShown = false;
        options.store();
    }

    function hideVersionSelection() {
        versionSelectShown = false;
        options.store();
    }

    let clientRunning = false;

    async function runClient() {
        console.log("Client started");
        log = [];
        clientRunning = true;

        let build = getBuild();
        console.debug("Running build", build);
        await invoke("run_client", { buildId: build.buildId, accountData: options.currentAccount, options: options, mods: mods });
    }

    listen("client-exited", () => {
        clientRunning = false;
    });

    listen("client-error", (e) => {
        alert(e.payload);
        clientLogShown = true;
        console.error(e.payload);
    });

    function updateModStates() {
        options.modStates = mods.reduce(function(map, mod) {
            map[mod.name] = mod.enabled;
            return map;
        }, {});

        console.debug("Updated mod states", options.modStates);
        options.store();
    }

    async function terminateClient() {
        await invoke("terminate");
    }

    function clearData() {
        invoke("clear_data", { options }).then(() => {
            alert("Data cleared.");
        }).catch(e => {
            alert("Failed to clear data: " + e);
            console.error(e)
        });
    }

    let dataFolderPath;
    invoke("default_data_folder_path").then(result => {
        dataFolderPath = result;
    }).catch(e => {
        alert("Failed to get data folder: " + e);
        console.error(e)
    });
</script>

{#if clientLogShown}
    <ClientLog messages={log} on:hideClientLog={() => clientLogShown = false} />
{/if}

{#if settingsShown}
    <SettingsContainer title="Settings" on:hideSettings={hideSettings}>
        <TextSetting title="JVM Location" placeholder="Internal" bind:value={options.customJavaPath} />
        <ToggleSetting title="Keep launcher running" bind:value={options.keepLauncherOpen} />
        <RangeSetting title="Memory" min={20} max={100} bind:value={options.memoryPercentage} valueSuffix="%" step={1} />
        <RangeSetting title="Concurrent Downloads" min={1} max={50} bind:value={options.concurrentDownloads} valueSuffix="connections" step={1} />
        <ButtonSetting text="Logout" on:click={() => dispatch("logout")} color="#4677FF" />
        <DirectorySelector title="Data Location" placeholder={dataFolderPath} bind:value={options.customDataPath} />
        <ButtonSetting text="Clear data" on:click={clearData} color="#B83529" />
    </SettingsContainer>
{/if}

{#if versionSelectShown}
    <SettingsContainer title="Select version" on:hideSettings={hideVersionSelection}>
        <SelectSetting title="Branch" items={branches.map(e => ({ value: e, text: e }))} bind:value={options.preferredBranch} on:change={requestBuilds} />
        <SelectSetting title="Build" items={[{ value: -1, text: "Latest" }, ...builds.filter(e => e.release || options.showNightlyBuilds).map(e => ({ value: e.buildId, text: e.lbVersion + " git-" + e.commitId.substring(0, 7) + " - " + e.date }))]} bind:value={options.preferredBuild} on:change={updateData} />
        <ToggleSetting title="Show nightly builds" bind:value={options.showNightlyBuilds} on:change={updateData} />
        <SettingWrapper title="Additional mods">
            {#each mods as m}
                {#if !m.required}
                    <ToggleSetting title={m.name} bind:value={m.enabled} on:change={updateModStates} />
                {/if}

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
                <ProgressStatus value={progressBarProgress} max={progressBarMax} text={progressBarLabel} />
            {/if}
        </StatusBar>
        <Account username={options.currentAccount.name} uuid={options.currentAccount.uuid} accountType={options.currentAccount.type} on:showSettings={() => settingsShown = true} />
        <ButtonClose />
    </TitleBar>

    <ContentWrapper>
        <LaunchArea {versionInfo} {lbVersion} {mcVersion} on:showVersionSelect={() => versionSelectShown = true} on:showClientLog={() => clientLogShown = true} on:launch={runClient} on:terminate={terminateClient} running={clientRunning} />
        <NewsArea />
    </ContentWrapper>
</VerticalFlexWrapper>
