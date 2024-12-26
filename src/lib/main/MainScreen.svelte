<script>
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
    import DirectorySelectorSetting from "../settings/DirectorySelectorSetting.svelte";
    import FileSelectorSetting from "../settings/FileSelectorSetting.svelte";
    import LauncherVersion from "../settings/LauncherVersion.svelte";
    import IconButtonSetting from "../settings/IconButtonSetting.svelte";
    import CustomModSetting from "../settings/CustomModSetting.svelte";
    import Tabs from "../settings/tab/Tabs.svelte";
    import Description from "../settings/Description.svelte";
    import LiquidBounceAccount from "../settings/LiquidBounceAccount.svelte";

    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { open as dialogOpen } from "@tauri-apps/plugin-dialog";
    import { open as shellOpen } from "@tauri-apps/plugin-shell";

    export let options;

    const dispatch = createEventDispatcher();

    const MIN_MEMORY = 2048;
    const WARNING_MEMORY = 4096;
    let systemMemory = 0;

    let dataState = {
        running: false,
        logShown: false,
        settingsShown: false,
        versionSelectShown: false,
        launchVersionWarningShown: false,
        launchVersionWarningCountdown: 0,
        branches: [],
        builds: [],
        currentBuild: null,
        recommendedMods: [],
        customMods: [],
        launcherVersion: "",
        defaultDataFolder: ""
    };

    let progressState = {
        max: 0,
        value: 0,
        text: ""
    };

    let activeSettingsTab = "General";
    let log = [];

    $: if (dataState.launchVersionWarningShown && dataState.launchVersionWarningCountdown > 0) {
        const countdown = setInterval(() => {
            dataState = {
                ...dataState,
                launchVersionWarningCountdown: dataState.launchVersionWarningCountdown - 1
            };

            if (dataState.launchVersionWarningCountdown <= 0) {
                clearInterval(countdown);
            }
        }, 1000);
    }

    $: if (options.version.branchName) {
        updateData().then(() => options.store());
    }

    function updateModStates() {
        const branchName = options.version.branchName;
        if (!options.version.options[branchName]) {
            options.version.options[branchName] = {
                modStates: {},
                customModStates: {}
            };
        }

        const branchOptions = options.version.options[branchName];

        dataState.recommendedMods.forEach(mod => {
            branchOptions.modStates[mod.name] = mod.enabled;
        });

        dataState.customMods.forEach(mod => {
            branchOptions.customModStates[mod.name] = mod.enabled;
        });

        options.store();
    }

    async function updateData() {
        try {
            const builds = await invoke("request_builds", {
                branch: options.version.branchName,
                release: !options.launcher.showNightlyBuilds
            });

            builds.forEach(build => {
                const date = new Date(build.date);
                build.date = date.toLocaleString();
                build.dateDay = date.toLocaleDateString();
            });

            dataState = { ...dataState, builds };

            const buildId = options.version.buildId;

            if (buildId !== -1 && !builds.find(build => build.buildId === buildId)) {
                options.version.buildId = -1;
                options.store();
            }

            const activeBuild = buildId === -1 ? builds[0] : builds.find(build => build.buildId === buildId);
            if (!activeBuild) return;

            const changelog = await invoke("fetch_changelog", {
                buildId: activeBuild.buildId
            });

            dataState = {
                ...dataState,
                currentBuild: { ...activeBuild, changelog: changelog.changelog }
            };

            await updateMods();
        } catch (error) {
            console.error("Failed to update data:", error);
            alert(`Failed to update data: ${error.message}`);
        }
    }

    async function updateMods() {
        const { currentBuild } = dataState;
        if (!currentBuild) return;

        try {
            const [recommendedMods, customMods] = await Promise.all([
                invoke("request_mods", {
                    mcVersion: currentBuild.mcVersion,
                    subsystem: currentBuild.subsystem
                }),
                invoke("get_custom_mods", {
                    branch: currentBuild.branch,
                    mcVersion: currentBuild.mcVersion
                })
            ]);

            const branchOptions = options.version.options[currentBuild.branch];

            if (branchOptions) {
                recommendedMods.forEach(mod => {
                    mod.enabled = branchOptions.modStates[mod.name] ?? mod.enabled;
                });

                customMods.forEach(mod => {
                    mod.enabled = branchOptions.customModStates[mod.name] ?? mod.enabled;
                });
            }

            dataState = {
                ...dataState,
                recommendedMods,
                customMods
            };
        } catch (error) {
            console.error("Failed to request mods:", error);
        }
    }

    async function authenticateClientAccount() {
        try {
            progressState = { ...progressState, text: "Authenticating client account..." };
            const account = await invoke("client_account_update", {
                account: options.premium.account
            });
            options.premium.account = account;
        } catch (e) {
            console.error("Failed to authenticate client account:", e);
        }
    }

    async function refreshMinecraftSession() {
        try {
            progressState = { ...progressState, text: "Refreshing minecraft session..." };
            const account = await invoke("refresh", {
                accountData: options.start.account
            });
            options.start.account = account;
        } catch (e) {
            console.error("Failed to refresh account:", e);
            alert("Failed to refresh account session: " + e + "\n\nYou have been logged out. Please try logging in again.");
            options.start.account = null;
            dataState = { ...dataState, running: false };
            throw e;
        }
    }

    async function runClient() {
        if (dataState.running) return;

        log = [];

        try {
            dataState = { ...dataState, running: true };
            progressState = { max: 0, value: 0, text: "Starting client..." };

            if (options.premium.account) {
                await authenticateClientAccount();
            }
            await refreshMinecraftSession();

            if (options.start.memory < WARNING_MEMORY) {
                const confirmed = await confirm(
                    `You are about to launch the client with less than ${WARNING_MEMORY} MiB of memory. This may cause performance issues. Do you want to continue?`
                );
                if (!confirmed) {
                    dataState = { ...dataState, running: false };
                    return;
                }
            }

            await options.store();

            await invoke("run_client", {
                buildId: dataState.currentBuild.buildId,
                options,
                mods: [...dataState.recommendedMods, ...dataState.customMods]
            });
        } catch (error) {
            console.error("Failed to start client:", error);
            log = [...log, `Failed to start client: ${error.message}`];
            dataState = { ...dataState, logShown: true, running: false };
        }
    }

    async function runClientWithWarning() {
        const isWarning = options.version.branchName === "legacy" ||
            (options.version.branchName === "nextgen" && options.version.buildId !== -1);

        if (isWarning) {
            dataState = {
                ...dataState,
                launchVersionWarningShown: true,
                launchVersionWarningCountdown: 3
            };
        } else {
            await runClient();
        }
    }

    async function switchToNextgen() {
        dataState = { ...dataState, launchVersionWarningShown: false };
        options.version.branchName = "nextgen";
        options.version.buildId = -1;
        await options.store();
        await updateData();
        await runClient();
    }

    async function runClientAnyway() {
        dataState = { ...dataState, launchVersionWarningShown: false };
        await runClient();
    }

    async function terminateClient() {
        await invoke("terminate");
    }

    async function clearData() {
        try {
            await invoke("clear_data", { options });
            alert("Data cleared.");
        } catch (error) {
            console.error("Failed to clear data:", error);
            alert(`Failed to clear data: ${error.message}`);
        }
    }

    async function handleCustomModDelete(event) {
        const { currentBuild } = dataState;

        try {
            await invoke("delete_custom_mod", {
                branch: currentBuild.branch,
                mcVersion: currentBuild.mcVersion,
                modName: `${event.detail.name}.jar`
            });

            await updateMods();
        } catch (error) {
            console.error("Failed to delete mod:", error);
            alert(`Failed to delete mod: ${error.message}`);
        }
    }

    async function handleInstallMod() {
        const { currentBuild } = dataState;

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
                        branch: currentBuild.branch,
                        mcVersion: currentBuild.mcVersion,
                        path: file
                    });
                }

                await updateMods();
            }
        } catch (error) {
            console.error("Failed to install mod:", error);
            alert(`Failed to install mod: ${error.message}`);
        }
    }

    async function authClientAccount() {
        try {
            const account = await invoke("client_account_authenticate");
            options.premium.account = account;
            await options.store();
        } catch (error) {
            console.error("Failed to authenticate client account:", error);
            alert(`Failed to authenticate client account: ${error.message}`);
        }
    }

    async function initialize() {
        try {
            const [branches, version, defaultDataFolder, sys_memory] = await Promise.all([
                invoke("request_branches"),
                invoke("get_launcher_version"),
                invoke("default_data_folder_path"),
                invoke("sys_memory")
            ]);

            systemMemory = sys_memory;

            dataState = {
                ...dataState,
                branches: branches.branches.sort((a, b) =>
                    (a === branches.defaultBranch ? -1 : b === branches.defaultBranch ? 1 : 0)),
                launcherVersion: version,
                defaultDataFolder
            };

            if (!options.version.branchName) {
                options.version.branchName = branches.defaultBranch;
                await options.store();
            }

            await updateData();
        } catch (error) {
            console.error("Initialization failed:", error);
            alert(`Failed to initialize: ${error.message}`);
        }
    }

    listen("process-output", (event) => {
        log = [...log, event.payload];
    });

    listen("progress-update", (event) => {
        const { type, value } = event.payload;
        switch (type) {
            case "max":
                progressState = { ...progressState, max: value };
                break;
            case "progress":
                progressState = { ...progressState, value };
                break;
            case "label":
                progressState = { ...progressState, text: value };
                break;
        }
    });

    listen("client-exited", () => {
        dataState = { ...dataState, running: false };
    });

    listen("client-error", () => {
        dataState = { ...dataState, logShown: true };
    });

    listen("auth_url", async (event) => {
        try {
            await shellOpen(event.payload);
        } catch (error) {
            console.error("Failed to open auth URL:", error);
        }
    });

    initialize();
</script>

{#if dataState.launchVersionWarningShown}
    <SettingsContainer
            title="You are about to launch an unsupported version!"
            on:hideSettings={() => dataState = { ...dataState, launchVersionWarningShown: false }}
    >
        <Description
                description="The selected version of LiquidBounce is no longer officially supported. We recommend upgrading to the latest version of LiquidBounce Nextgen, which works with all Minecraft versions from 1.7 onward."
        />
        <ButtonSetting
                text="Switch to Nextgen now"
                on:click={switchToNextgen}
                color="#4677FF"
        />
        <ButtonSetting
                disabled={dataState.launchVersionWarningCountdown > 0}
                text="Launch anyway{dataState.launchVersionWarningCountdown > 0 ? ` (${dataState.launchVersionWarningCountdown})` : ''}"
                on:click={runClientAnyway}
                color="#B83529"
        />
    </SettingsContainer>
{/if}

{#if dataState.logShown}
    <ClientLog
            messages={log}
            on:hideClientLog={() => dataState = { ...dataState, logShown: false }}
    />
{/if}

{#if dataState.settingsShown}
    <SettingsContainer
            title="Settings"
            on:hideSettings={() => {
            dataState = { ...dataState, settingsShown: false };
            options.store();
        }}
    >
        <Tabs
                tabs={["General", "Donator"]}
                bind:activeTab={activeSettingsTab}
                slot="tabs"
        />

        {#if activeSettingsTab === "General"}
            <SelectSetting
                    title="JVM Distribution"
                    items={[
                    { value: "automatic", text: "Automatic" },
                    { value: "manual", text: "Manual" },
                    { value: "custom", text: "Custom" }
                ]}
                    bind:value={options.start.javaDistribution.type}
            />

            {#if options.start.javaDistribution.type === "manual"}
                <SelectSetting
                        title="Distribution"
                        items={[
                        { value: "temurin", text: "Eclipse Temurin" },
                        { value: "graalvm", text: "GraalVM" }
                    ]}
                        bind:value={options.start.javaDistribution.value}
                />
            {/if}

            {#if options.start.javaDistribution.type === "custom"}
                <FileSelectorSetting
                        title="Custom JVM Path"
                        placeholder="Select Java wrapper location"
                        bind:value={options.start.javaDistribution.value}
                        filters={[{ name: "javaw", extensions: [] }]}
                        windowTitle="Select custom Java wrapper"
                />
            {/if}

            <DirectorySelectorSetting
                    title="Data Location"
                    placeholder={dataState.defaultDataFolder}
                    bind:value={options.start.customDataPath}
                    windowTitle="Select custom data directory"
            />
            <RangeSetting
                    title="Memory"
                    min={MIN_MEMORY}
                    max={systemMemory}
                    bind:value={options.start.memory}
                    valueSuffix=" MiB"
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
            <ButtonSetting
                    text="Sign out of Minecraft Account"
                    on:click={() => dispatch("logout")}
                    color="#4677FF"
            />
            <ButtonSetting
                    text="Clear Minecraft Game Data"
                    on:click={clearData}
                    color="#B83529"
            />
            <LauncherVersion version={dataState.launcherVersion} />
        {:else if activeSettingsTab === "Donator"}
            <ToggleSetting
                    title="Skip Advertisements"
                    disabled={!options.premium.account || !options.premium.account.premium}
                    bind:value={options.premium.skipAdvertisement}
            />

            {#if options.premium.account}
                <SettingWrapper title="Account Information">
                    <LiquidBounceAccount account={options.premium.account} />
                </SettingWrapper>

                {#if !options.premium.account.premium}
                    <Description
                            description="There appears to be no donation associated with this account. Please link it on the account management page."
                    />
                {/if}

                <ButtonSetting
                        text="Manage Account"
                        on:click={() => shellOpen("https://user.liquidbounce.net")}
                        color="#4677FF"
                />
                <ButtonSetting
                        text="Logout"
                        on:click={() => options.premium.account = null}
                        color="#B83529"
                />
            {:else}
                <Description
                        description="By donating, you not only support the ongoing development of the client but also receive a donator cape and the ability to bypass ads on the launcher."
                />

                <ButtonSetting
                        text="Login with LiquidBounce Account"
                        on:click={authClientAccount}
                        color="#4677FF"
                />
            {/if}
        {/if}
    </SettingsContainer>
{/if}

{#if dataState.versionSelectShown}
    <SettingsContainer
            title="Select version"
            on:hideSettings={() => {
            dataState = { ...dataState, versionSelectShown: false };
            options.store();
        }}
    >
        <SelectSetting
                title="Branch"
                items={dataState.branches.map(e => ({
                value: e,
                text: `${e.charAt(0).toUpperCase()}${e.slice(1)} ${e === "legacy" ? "(unsupported)" : ""}`
            }))}
                bind:value={options.version.branchName}
        />
        <SelectSetting
                title="Build"
                items={[
                { value: -1, text: "Latest" },
                ...dataState.builds
                    .map(e => ({
                        value: e.buildId,
                        text: `${e.lbVersion} git-${e.commitId.substring(0, 7)} - ${e.date}`
                    }))
            ]}
                bind:value={options.version.buildId}
        />
        <ToggleSetting
                title="Show nightly builds"
                bind:value={options.launcher.showNightlyBuilds}
                disabled={false}
                on:change={updateData}
        />
        <SettingWrapper title="Recommended mods">
            {#each dataState.recommendedMods as mod}
                <ToggleSetting
                        title={mod.name}
                        bind:value={mod.enabled}
                        disabled={mod.required}
                        on:change={updateModStates}
                />
            {/each}
        </SettingWrapper>
        <SettingWrapper title={`Additional mods for ${dataState.currentBuild?.branch} ${dataState.currentBuild?.mcVersion}`}>
            <div slot="title-element">
                <IconButtonSetting
                        text="Install"
                        icon="icon-plus"
                        on:click={handleInstallMod}
                />
            </div>

            {#each dataState.customMods as mod}
                <CustomModSetting
                        title={mod.name}
                        bind:value={mod.enabled}
                        on:change={updateModStates}
                        on:delete={handleCustomModDelete}
                />
            {/each}
        </SettingWrapper>
    </SettingsContainer>
{/if}

<VerticalFlexWrapper
        blur={dataState.settingsShown || dataState.versionSelectShown ||
          dataState.logShown || dataState.launchVersionWarningShown}
>
    <TitleBar>
        <Logo />
        <StatusBar>
            {#if !dataState.running}
                <TextStatus
                        text="Welcome back, {options.start.account?.name}."
                />
            {:else}
                <ProgressStatus {...progressState} />
            {/if}
        </StatusBar>
        <Account
                username={options.start.account?.name}
                uuid={options.start.account?.id}
                accountType={options.start.account?.type}
                on:showSettings={() => dataState = { ...dataState, settingsShown: true }}
        />
        <ButtonClose />
    </TitleBar>

    <ContentWrapper>
        <LaunchArea
                versionInfo={{
                bannerUrl: "img/banner.png",
                title: dataState.currentBuild ?
                    `LiquidBounce ${dataState.currentBuild.lbVersion} for Minecraft ${dataState.currentBuild.mcVersion}` :
                    "Loading...",
                date: dataState.currentBuild?.dateDay || "Loading...",
                description: dataState.currentBuild?.changelog || "Loading..."
            }}
                lbVersion={dataState.currentBuild?.lbVersion}
                mcVersion={dataState.currentBuild?.mcVersion}
                on:showVersionSelect={() => dataState = { ...dataState, versionSelectShown: true }}
                on:showClientLog={() => dataState = { ...dataState, logShown: true }}
                on:launch={runClientWithWarning}
                on:terminate={terminateClient}
                running={dataState.running}
        />
        <NewsArea />
    </ContentWrapper>
</VerticalFlexWrapper>