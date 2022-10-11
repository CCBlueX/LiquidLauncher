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

    let login = true;
    let optionsShown = false;
    let accountData; // username, accessToken, id, type

    // Logins

    function loginIntoMojang(username, password) {

        function onDone(account) {
            accountData = account;
            login = false;
        }

        function onError(error) {
            console.log("Error: " + error);

            // todo: handle login error
            // label.textContent = "Error: " + error;
        }

        Window.this.xcall("login_mojang", username, password, onError, onDone);
    }

    function loginIntoOffline(username) {
        if (username.length <= 0 || username.length > 16) {
            // Username is too long
            // todo: handle error
            console.log("Not valid username.");
            return;
        }

        accountData = {
            "username": username,
            "accessToken": "-",
            "id": "", // todo: get uuid from username
            "type": "legacy"
        };
        login = false;
    }

    function loginIntoMicrosoft() {
        // see microsoft_login branch
        // Window.this.xcall("login_microsoft");
    }

    function exitApp() {
        Window.this.xcall("exit_app");
    }

    function switchOptions() {
        optionsShown = !optionsShown;
    }
</script>

<main>
    {#if login}
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
            <WelcomeMessage message="Welcome {accountData.username}, try out our new version!" />
            <Account accountName={accountData.username} accountType="Premium" avatarUrl="https://visage.surgeplay.com/face/{accountData.id}" showOptions={switchOptions} />
            <ButtonClose exit={exitApp} />
        </TitleBar>

        <Content>
            <LaunchArea accountData={accountData} />

            {#if optionsShown}
                <Options closeOptions={switchOptions} />
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