<!-- lib/main/MainScreen.svelte -->
<script>
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import VerticalFlexWrapper from "../common/VerticalFlexWrapper.svelte";
    import MainHeader from "./MainHeader.svelte";
    import ContentWrapper from "./ContentWrapper.svelte";
    import LaunchArea from "./LaunchArea.svelte";
    import NewsArea from "./news/NewsArea.svelte";
    import VersionWarning from "./VersionWarning.svelte";
    import ClientLog from "./log/ClientLog.svelte";
    import Settings from "./settings/Settings.svelte";
    import VersionSelect from "./VersionSelect.svelte";
    import {createEventDispatcher, onMount} from "svelte";

    const dispatch = createEventDispatcher();

    export let options;

    let running = false;

    let logShown = false;
    let settingsShown = false;
    let versionSelectShown = false;
    let launchVersionWarningShown = false;
    let launchVersionWarningCountdown = 0;
    let log = [];

    let versionState = {
        branches: [],
        builds: [],
        currentBuild: null,
        recommendedMods: [],
        customMods: []
    };

    let progressState = {
        max: 0,
        value: 0,
        text: ""
    };

    $: if (launchVersionWarningShown && launchVersionWarningCountdown > 0) {
        const countdown = setInterval(() => {
            launchVersionWarningCountdown--;
            if (launchVersionWarningCountdown <= 0) clearInterval(countdown);
        }, 1000);
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
        versionState.recommendedMods.forEach(mod => {
            branchOptions.modStates[mod.name] = mod.enabled;
        });
        versionState.customMods.forEach(mod => {
            branchOptions.customModStates[mod.name] = mod.enabled;
        });
        options.store();
    }

    async function updateData() {
        const newBuilds = await invoke("request_builds", {
            branch: options.version.branchName,
            release: !options.launcher.showNightlyBuilds
        });

        newBuilds.forEach(build => {
            const date = new Date(build.date);
            build.date = date.toLocaleString();
            build.dateDay = date.toLocaleDateString();
        });

        versionState.builds = newBuilds;
        const buildId = options.version.buildId;

        if (buildId !== -1 && !versionState.builds.find(build => build.buildId === buildId)) {
            options.version.buildId = -1;
            await options.store();
        }

        const activeBuild = buildId === -1 ? versionState.builds[0] : versionState.builds.find(build => build.buildId === buildId);
        if (!activeBuild) return;

        const changelog = await invoke("fetch_changelog", {
            buildId: activeBuild.buildId
        });

        versionState.currentBuild = { ...activeBuild, changelog: changelog.changelog };
        await updateMods();
    }

    async function updateMods() {
        if (!versionState.currentBuild) return;

        const [newRecommendedMods, newCustomMods] = await Promise.all([
            invoke("request_mods", {
                mcVersion: versionState.currentBuild.mcVersion,
                subsystem: versionState.currentBuild.subsystem
            }),
            invoke("get_custom_mods", {
                branch: versionState.currentBuild.branch,
                mcVersion: versionState.currentBuild.mcVersion
            })
        ]);

        const branchOptions = options.version.options[versionState.currentBuild.branch];

        if (branchOptions) {
            newRecommendedMods.forEach(mod => {
                mod.enabled = branchOptions.modStates[mod.name] ?? mod.enabled;
            });
            newCustomMods.forEach(mod => {
                mod.enabled = branchOptions.customModStates[mod.name] ?? mod.enabled;
            });
        }

        versionState.recommendedMods = newRecommendedMods;
        versionState.customMods = newCustomMods;
    }

    async function runClientWithWarning() {
        const isWarning = options.version.branchName === "legacy" ||
            (options.version.branchName === "nextgen" && options.version.buildId !== -1);

        if (isWarning) {
            launchVersionWarningShown = true;
            launchVersionWarningCountdown = 3;
        } else {
            await runClient();
        }
    }

    const WARNING_MEMORY = 4096;

    async function runClient() {
        if (running) return;

        log = [];

        try {
            running = true;
            progressState = { max: 0, value: 0, text: "Starting client..." };

            await authenticate();
            await checkMemory();
            await launchClient();
        } catch (error) {
            console.error("Failed to start client:", error);
            log = [...log, `Failed to start client: ${error}`];
            running = false;
            logShown = true;
        }
    }

    async function authenticate() {
        if (options.premium.account) {
            try {
                progressState.text = "Authenticating client account...";
                options.premium.account = await invoke("client_account_update", {
                    account: options.premium.account
                });
            } catch (e) {
                console.error("Failed to authenticate client account:", e);
                log = [...log, `Failed to authenticate client account: ${e}`];
                options.premium.account = null;
            }
        }

        progressState.text = "Refreshing minecraft session...";
        try {
            options.start.account = await invoke("refresh", {
                accountData: options.start.account
            });
        } catch (e) {
            options.start.account = null;
            throw e;
        }
    }

    async function checkMemory() {
        if (options.start.memory < WARNING_MEMORY) {
            const confirmed = await confirm(
                `You are about to launch the client with less than ${WARNING_MEMORY} MB of memory. This may cause performance issues. Do you want to continue?`
            );

            if (!confirmed) {
                running = false;
                throw new Error("Memory warning declined");
            }
        }
    }

    async function launchClient() {
        await options.store();
        await invoke("run_client", {
            buildId: versionState.currentBuild.buildId,
            options,
            mods: [...versionState.recommendedMods, ...versionState.customMods]
        });
    }

    async function terminateClient() {
        await invoke("terminate");
    }

    async function switchToNextgen() {
        launchVersionWarningShown = false;
        options.version.branchName = "nextgen";
        options.version.buildId = -1;
        await options.store();
        await updateData();
        await runClient();
    }

    listen("process-output", (event) => {
        log = [...log, event.payload];
    });

    listen("progress-update", (event) => {
        const { type, value } = event.payload;
        switch (type) {
            case "max":
                progressState.max = value;
                break;
            case "progress":
                progressState.value = value;
                break;
            case "label":
                progressState.text = value;
                break;
        }
    });

    listen("client-exited", () => {
        running = false;
    });

    listen("client-error", () => {
        logShown = true;
    });

    onMount(async () => {
        let branchesData = await invoke("request_branches");
        versionState.branches = branchesData.branches.sort((a, b) =>
            (a === branchesData.defaultBranch ? -1 : b === branchesData.defaultBranch ? 1 : 0));

        if (!options.version.branchName || !versionState.branches.includes(options.version.branchName)) {
            options.version.branchName = branchesData.defaultBranch;
            await options.store();
        }

        await updateData();
    });
</script>

{#if launchVersionWarningShown}
    <VersionWarning
            {launchVersionWarningCountdown}
            on:switchToNextgen={switchToNextgen}
            on:runClientAnyway={async () => {
                launchVersionWarningShown = false;
                await runClient();
            }}
            on:hide={() => launchVersionWarningShown = false}
    />
{/if}

{#if logShown}
    <ClientLog messages={log} on:hideClientLog={() => logShown = false} />
{/if}

{#if settingsShown}
    <Settings
            bind:options
            on:hide={async () => {
                settingsShown = false;
                await options.store();
            }}
    />
{/if}

{#if versionSelectShown}
    <VersionSelect
            bind:options
            {versionState}
            on:updateData={updateData}
            on:updateModStates={updateModStates}
            on:updateMods={updateMods}
            on:hide={async () => {
            versionSelectShown = false;
            await options.store();
        }}
    />
{/if}

<VerticalFlexWrapper
        blur={settingsShown || versionSelectShown || logShown || launchVersionWarningShown}
>
    <MainHeader
            account={options.start.account}
            {running}
            {progressState}
            on:showSettings={() => settingsShown = true}
    />

    <ContentWrapper>
        <LaunchArea
                versionInfo={{
                    bannerUrl: "img/banner.png",
                    title: versionState.currentBuild ?
                        `LiquidBounce ${versionState.currentBuild.lbVersion} for Minecraft ${versionState.currentBuild.mcVersion}` :
                        "Loading...",
                    date: versionState.currentBuild?.dateDay || "Loading...",
                    description: versionState.currentBuild?.changelog || "Loading..."
                }}
                mcVersion={versionState.currentBuild?.mcVersion || "Loading..."}
                lbVersion={versionState.currentBuild?.lbVersion || "Loading..."}
                {running}
                on:showVersionSelect={() => versionSelectShown = true}
                on:showClientLog={() => logShown = true}
                on:launch={runClientWithWarning}
                on:terminate={terminateClient}
        />
        <NewsArea />
    </ContentWrapper>
</VerticalFlexWrapper>