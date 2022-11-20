<script>
    import About from "./login/About.svelte";
    import Login from "./login/Login.svelte";
    import NewsContainer from "./main/content/changelogs/NewsContainer.svelte";
    import Content from "./main/content/Content.svelte";
    import LaunchArea from "./main/content/LaunchArea.svelte";
    import Account from "./main/title-bar/Account.svelte";
    import ButtonClose from "./main/title-bar/ButtonClose.svelte";
    import Logo from "./main/title-bar/Logo.svelte";
    import Spacer from "./main/title-bar/Spacer.svelte";
    import TitleBar from "./main/title-bar/TitleBar.svelte";
    import WelcomeMessage from "./main/title-bar/WelcomeMessage.svelte";
    import Options from "./main/content/Options.svelte";
    import UpdateContainer from "./main/content/UpdateContainer.svelte";

    // When using the Tauri API npm package:
    import { invoke } from '@tauri-apps/api/tauri'

    // Options Storage
    let optionsShown = false;

    console.log("Loading options...");
    var options = null;
    invoke('get_options')
        .then((o) => {
            options = o;
            console.log(JSON.stringify(o));
        })
        .catch((e) => console.error(e))

    // Versions
    let versionData;

    let branches = [];
    let builds = [];

    let mods = [];

    function updateBranches() {
        invoke('request_branches')
            .then((receivedBranches) => {
                branches = receivedBranches;

                let branch;
                if (branches.includes(options.preferredBranch)) {
                    branch = options.preferredBranch;
                } else {
                    branch = branches[0];
                }
                updateBuilds(branch);
            })
            .catch((e) => console.error(e))
    }

    function updateBuilds(branch) {
        invoke('request_builds', { branch: branch })
            .then((receivedBuilds) => {
                builds = receivedBuilds;

                let build = builds.find(x => x.buildId === options.preferredBuild);
                if (build === undefined) {
                    build = builds.find(x => x.release || options.showNightlyBuilds); // Choose newest version
                }
                versionData = build;

                requestMods();
            })
            .catch((e) => console.error(e))
    }

    function requestMods() {
        // function onResponse(listOfMods) {
        //     mods = listOfMods;

        //     mods.forEach(x => {
        //         x.enabled = !options.disabledMods.includes(x.name);
        //     });
        // }

        // function onError(e) {
        //     console.error("unable to update mods", e);
        // }

        // Window.this.xcall("get_mods", versionData.mcVersion, versionData.subsystem, onResponse, onError);
    }

    updateBranches();

    // Account
    let accountData = options.currentAccount; // name, token, uuid, type

    function saveAccount(account) {
        accountData = account;
        options.currentAccount = accountData;
        Window.this.xcall("store_options", options);

        console.log("Successfully saved account.");
        console.log(JSON.stringify(accountData));
    }

    function loginIntoMojang(username, password) {

        function onError(error) {
            console.error("failed mojang authentication", error);
        }

        // Window.this.xcall("login_mojang", username, password, onError, saveAccount);
    }

    function loginIntoOffline(username) {
        if (username.length <= 0 || username.length > 16) {
            // Username is too long
            // todo: handle error
            console.error("not valid username (0 < username <= 16)", username);
            return;
        }

        // Window.this.xcall("login_offline", username, saveAccount);
    }

    function loginIntoMicrosoft(onCode) {

        function onError(error) {
            console.error("failed microsoft authentication", error);
        }

        // Window.this.xcall("login_msa", onError, onCode, saveAccount);
    }

    function logout() {
        // Window.this.xcall("logout", accountData);

        accountData = null;
        options.currentAccount = accountData;
        // Window.this.xcall("store_options", options);
    }

    // Refresh authentication
    if (accountData != null) {
        console.log("Refreshing account data...");

        function onError(error) {
            console.error("failed refresh authentication", error);
            logout();
        }

        // Window.this.xcall("refresh_account", accountData, onError, saveAccount);
    }

    // App

    function exitApp() {
        invoke('exit_app')
    }

    function switchOptions() {
        optionsShown = !optionsShown;
    }

    // function checkForUpdates() {
    //     function newerVersionFound(data) {
    //         console.log(JSON.stringify(data));
    //         updateData = data;
    //     }

    //     console.log("Checking for app updates...");
    //     Window.this.xcall("check_for_updates", newerVersionFound);
    // }

    // // Check for updates at start-up
    // checkForUpdates();
</script>

<main>
    {#if accountData == null}
        <TitleBar>
            <Logo />
            <Spacer />
            <ButtonClose exit={exitApp} />
        </TitleBar>

        <Content>
            <About />
            <Login handleMojangLogin={loginIntoMojang} handleMicrosoftLogin={loginIntoMicrosoft} handleOfflineLogin={loginIntoOffline} />
        </Content>
    {:else}
        <TitleBar>
            <Logo />
            <WelcomeMessage message="Welcome {accountData.name}, try out our new version!" />
            <Account accountName={accountData.name} accountType={accountData.type} avatarUrl="https://visage.surgeplay.com/face/{accountData.uuid}" showOptions={switchOptions} />
            <ButtonClose exit={exitApp} />
        </TitleBar>


            <Content>
                <LaunchArea accountData={accountData} options={options} versionData={versionData} mods={mods} />

                {#if optionsShown}
                    <!-- <Options bind:options={options} logout={logout} bind:versionData={versionData} branches={branches} builds={builds} bind:mods={mods} updateBuilds={updateBuilds} /> -->
                {:else}
                    <NewsContainer />
                {/if}
            </Content>

    {/if}
</main>

<style>
	main {
        background-color: rgba(0, 0, 0, .68);
        height: 1*;
        padding: 32px;
	}
</style>