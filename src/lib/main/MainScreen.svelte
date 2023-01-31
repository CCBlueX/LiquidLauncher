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

    export let options;

    const dispatch = createEventDispatcher();

    const versionInfo = {
        bannerUrl: "img/b73.jpg",
        title: "Lorem ipsum dolor sit amet",
        date: "2021-05-07",
        description:
            "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua.\n\nAt vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua.\n\nAt vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet.",
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
        if (options.preferredBuild == -1) {
            return builds[0]; // get latest build
        }

        return builds.find((build) => build.buildId === options.preferredBuild);
    }

    let lbVersion = {};
    let mcVersion = {};

    let mods = [];

    function requestBuilds() {
        invoke("request_builds", { branch: options.preferredBranch })
            .then(b => {
                builds = b;
                // options.preferredBuild = b[0].buildId;

                updateData();
            })
            .catch(e => console.error(e));
    }

    function updateData() {
        let b = getBuild();

        lbVersion = {
            date: b.date,
            title: b.lbVersion
        };
        mcVersion = {
            date: "nix",
            title: b.mcVersion
        };

        requestMods(b.mcVersion, b.subsystem);
    }

    function requestMods(mcVersion, subsystem) {
        invoke("request_mods", { mcVersion, subsystem })
            .then(b => {
                mods = b;
            })
            .catch(e => console.error(e));
    }

    invoke("request_branches")
        .then(b => {
            branches = b;
            if (options.preferredBranch === null) {
                options.preferredBranch = b[0];
            }
            
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

    async function terminateClient() {
        await invoke("terminate");
    }
</script>

{#if clientLogShown}
    <ClientLog messages={log} on:hideClientLog={() => clientLogShown = false} />
{/if}

{#if settingsShown}
    <SettingsContainer title="Settings" on:hideSettings={hideSettings}>
        <TextSetting title="JVM Location" placeholder="Internal" bind:value={options.customJavaPath} ></TextSetting>
        <ToggleSetting title="Keep launcher running" bind:value={options.keepLauncherOpen} />
        <RangeSetting title="Memory" min={20} max={100} bind:value={options.memoryPercentage} valueSuffix="%" step={1}></RangeSetting>
        <ButtonSetting text="Logout" on:click={() => dispatch("logout")} />
    </SettingsContainer>
{/if}

{#if versionSelectShown}
    <SettingsContainer title="Select version" on:hideSettings={hideVersionSelection}>
        <SelectSetting title="Branch" items={branches.map(e => ({ value: e, text: e }))} bind:value={options.preferredBranch} on:change={requestBuilds}></SelectSetting>
        <SelectSetting title="Build" items={[{ value: -1, text: "Latest" }, ...builds.map(e => ({ value: e.buildId, text: e.lbVersion + " git-" + e.commitId.substring(0, 7) }))]} bind:value={options.preferredBuild} on:change={updateData}></SelectSetting>
        <ToggleSetting title="Show nightly builds" bind:value={options.showNightlyBuilds} />
        <SettingWrapper title="Additional mods">
            {#each mods as m}
                <ToggleSetting title={m.name} bind:value={m.enabled} />
            {/each}
        </SettingWrapper>
    </SettingsContainer>
{/if}

<VerticalFlexWrapper blur={settingsShown || versionSelectShown || clientLogShown}>
    <TitleBar>
        <Logo />
        <StatusBar>
            {#if !clientRunning}
                <TextStatus text="Welcome {options.currentAccount.name}, try out our new version!" />
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
