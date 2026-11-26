<!-- lib/main/MainScreen.svelte -->
<script>
    import {invoke} from "@tauri-apps/api/core";
    import {listen} from "@tauri-apps/api/event";
    import {confirm} from "@tauri-apps/plugin-dialog";
    import VerticalFlexWrapper from "../common/VerticalFlexWrapper.svelte";
    import MainHeader from "./MainHeader.svelte";
    import ContentWrapper from "./ContentWrapper.svelte";
    import LaunchArea from "./LaunchArea.svelte";
    import NewsArea from "./news/NewsArea.svelte";
    import VersionWarning from "./VersionWarning.svelte";
    import ClientLog from "./log/ClientLog.svelte";
    import Settings from "./settings/Settings.svelte";
    import FirstRunWarning from "./FirstRunWarning.svelte";
    import SettingsContainer from "../settings/SettingsContainer.svelte";
    import SignIn from "../accounts/SignIn.svelte";
    import {accounts, loadAccounts} from "../accounts/accounts.js";
    import {onMount} from "svelte";

    export let client;
    export let options;
    export let error;

    let clientAccount = null;

    let instances = [];
    let instance = null;
    let launching = false;
    $: starting = launching || instances.some(listed => listed.state === "starting");

    let logs = {};
    let logShown = null;
    let settingsShown = false;
    let addAccountShown = false;
    let settingsTab = "General";
    let launchVersionWarningShown = false;
    let firstRunWarningShown = false;
    let launchVersionWarningCountdown = 0;

    let versionState = {
        currentBuild: null,
        recommendedMods: [],
        customMods: []
    };

    let progressState = {
        max: 0,
        value: 0,
        text: "",
        speed: 0
    };

    $: if (launchVersionWarningShown && launchVersionWarningCountdown > 0) {
        const countdown = setInterval(() => {
            launchVersionWarningCountdown--;
            if (launchVersionWarningCountdown <= 0) clearInterval(countdown);
        }, 1000);
    }

    function updateModStates() {
        const branchName = "nextgen"; // todo: remove this later
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

    let buildRequest = 0;

    async function updateData() {
        const current = ++buildRequest;
        let build;
        try {
            build = await invoke("request_build", { client, options });
        } catch (e) {
            console.error("Failed to request the build:", e);
            error = {
                message: "Failed to establish connection with LiquidBounce API",
                error: e
            };
            return;
        }
        if (current !== buildRequest) return;

        // The chosen build is gone.
        if (!build) {
            options.version.buildId = -1;
            await options.store();
            return updateData();
        }

        const changelog = await invoke("fetch_changelog", {
            client,
            buildId: build.buildId
        });
        if (current !== buildRequest) return;

        versionState.currentBuild = {
            ...build,
            dateDay: new Date(build.date).toLocaleDateString(),
            changelog: changelog.changelog
        };
        await updateMods();
    }

    // The Client tab asks Modrinth about the custom mods once they are read from disk.
    $: clientTabShown = settingsShown && settingsTab === "Client";
    $: if (clientTabShown) updateMods();

    let modsRequest = 0;

    async function updateMods() {
        const build = versionState.currentBuild;
        if (!build) return;

        const current = ++modsRequest;
        const customMods = check => invoke("get_custom_mods", {
            options,
            branch: build.branch,
            mcVersion: build.mcVersion,
            subsystem: build.subsystem,
            check
        });

        const [newRecommendedMods, newCustomMods] = await Promise.all([
            invoke("request_mods", {
                client,
                mcVersion: build.mcVersion,
                subsystem: build.subsystem
            }),
            customMods(false)
        ]);
        if (current !== modsRequest) return;

        const branchOptions = options.version.options[build.branch];

        if (branchOptions) {
            newRecommendedMods.forEach(mod => {
                mod.enabled = branchOptions.modStates[mod.name] ?? mod.enabled;
            });
        }

        versionState.recommendedMods = newRecommendedMods;
        versionState.customMods = newCustomMods;

        if (clientTabShown) {
            const checkedCustomMods = await customMods(true);
            if (current === modsRequest) versionState.customMods = checkedCustomMods;
        }
    }

    async function runClientWithWarning() {
        if (!versionState.currentBuild) return;
        const isWarning = options.version.buildId !== -1;

        if (isWarning) {
            launchVersionWarningShown = true;
            launchVersionWarningCountdown = 3;
        } else {
            await runClient();
        }
    }

    const WARNING_MEMORY = 4096;

    async function runClient() {
        if (options.launcher.firstRun) {
            firstRunWarningShown = true;
            return;
        }

        if (starting) return;

        try {
            launching = true;
            progressState = { max: 0, value: 0, text: "Starting client...", speed: 0 };

            await authenticate();
            await checkMemory();
            const id = await launchClient();
            await refreshInstances();
            instance = instances.find(listed => listed.id === id) ?? null;
        } catch (error) {
            console.error("Failed to start client:", error);
            logs[0] = [`Failed to start client: ${error}`];
            logShown = 0;
        } finally {
            launching = false;
        }
    }

    async function refreshInstances() {
        instances = await invoke("list_instances");
        instance = instances.find(listed => listed.id === instance?.id) ?? null;
    }

    async function updateClientAccount() {
        try {
            clientAccount = await invoke("client_account", {client, options});
        } catch (e) {
            console.error("Failed to load client account:", e);
        }
    }

    async function authenticate() {
        progressState.text = "Refreshing minecraft session...";
        try {
            options.start.account = await invoke("refresh", {
                client,
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
                throw new Error("Memory warning declined");
            }
        }
    }

    async function launchClient() {
        await options.store();
        return invoke("run_client", {
            client,
            buildId: versionState.currentBuild.buildId,
            options,
            mods: [...versionState.recommendedMods, ...versionState.customMods]
        });
    }

    async function terminateClient() {
        await invoke("terminate", { id: instance.id });
    }

    function hideLog() {
        if (!instances.some(listed => listed.id === logShown)) delete logs[logShown];
        logShown = null;
    }

    function showSettings(tab) {
        settingsTab = tab;
        settingsShown = true;
    }

    async function hideSettings() {
        settingsShown = false;
        await options.store();
        // The data location may have changed, which has its own custom mods and accounts
        loadAccounts(options);
        await updateMods();
    }

    async function switchAccount(entry) {
        try {
            options.start.account = await invoke("select_account", {options, id: entry.id});
            await options.store();
        } catch (e) {
            alert("Failed to switch account.\n\n" + e);
        }
        await loadAccounts(options);
    }

    async function removeAccount(entry) {
        const confirmed = await confirm(
            `Remove ${entry.name}? It disappears from LiquidBounce's account manager too.`
        );
        if (!confirmed) return;

        try {
            await invoke("remove_account", {options, id: entry.id});
        } catch (e) {
            alert("Failed to remove account.\n\n" + e);
        }
        await loadAccounts(options);
    }

    async function accountAdded() {
        addAccountShown = false;
        await loadAccounts(options);
    }

    async function continueAfterFirstRun() {
        await hideFirstRunWarning();
        await runClient();
    }

    async function hideFirstRunWarning() {
        firstRunWarningShown = false;
        if (options.launcher.firstRun) {
            options.launcher.firstRun = false;
            await options.store();
        }
    }
    
    async function switchToNextgen() {
        launchVersionWarningShown = false;
        options.version.buildId = -1;
        await options.store();
        await updateData();
        await runClient();
    }

    listen("process-output", (event) => {
        const { id, line } = event.payload;
        logs[id] = [...(logs[id] ?? []), line];
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
            case "speed":
                progressState.speed = value;
                break;
        }
    });

    listen("client-running", refreshInstances);

    listen("client-exited", (event) => {
        const { id } = event.payload;
        if (logShown !== id) delete logs[id];
        refreshInstances();
        updateClientAccount();
        // The client writes the list when it exits.
        loadAccounts(options);
    });

    listen("client-error", (event) => {
        logShown = event.payload.id;
    });

    onMount(async () => {
        loadAccounts(options);
        updateClientAccount();
        refreshInstances();
        await updateData();
    });
</script>

{#if firstRunWarningShown}
    <FirstRunWarning
            on:hide={hideFirstRunWarning}
            on:continue={continueAfterFirstRun}
    />
{/if}

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

{#if logShown !== null}
    <ClientLog messages={logs[logShown] ?? []} on:hideClientLog={hideLog} />
{/if}

{#if settingsShown}
    <Settings
            {client}
            bind:options
            bind:clientAccount
            bind:activeTab={settingsTab}
            {versionState}
            on:updateData={updateData}
            on:updateModStates={updateModStates}
            on:updateMods={updateMods}
            on:hide={hideSettings}
    />
{/if}

{#if addAccountShown}
    <SettingsContainer title="Add account" on:hideSettings={() => addAccountShown = false}>
        <SignIn bind:options on:added={accountAdded} />
    </SettingsContainer>
{/if}

<VerticalFlexWrapper
        blur={settingsShown || addAccountShown || logShown !== null || launchVersionWarningShown || firstRunWarningShown}
>
    <MainHeader
            account={options.start.account}
            accounts={$accounts}
            accountsLocked={instances.length > 0}
            running={starting}
            {progressState}
            on:showSettings={() => showSettings("General")}
            on:addAccount={() => addAccountShown = true}
            on:switchAccount={(e) => switchAccount(e.detail)}
            on:removeAccount={(e) => removeAccount(e.detail)}
    />

    <ContentWrapper>
        <LaunchArea
                versionInfo={{
                    bannerUrl: "img/banner.png",
                    title: versionState.currentBuild ?
                        `LiquidBounce v${versionState.currentBuild.lbVersion}` :
                        "Loading...",
                    date: versionState.currentBuild?.dateDay || "Loading...",
                    description: versionState.currentBuild?.changelog || "Loading..."
                }}
                mcVersion={versionState.currentBuild?.mcVersion || "Loading..."}
                lbVersion={versionState.currentBuild?.lbVersion || "Loading..."}
                canLaunch={!!versionState.currentBuild && !starting}
                {instances}
                bind:instance
                on:showVersionSelect={() => showSettings("Client")}
                on:showClientLog={() => logShown = instance.id}
                on:launch={runClientWithWarning}
                on:terminate={terminateClient}
        />
        <NewsArea {client} />
    </ContentWrapper>
</VerticalFlexWrapper>
