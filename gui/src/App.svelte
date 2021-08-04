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

    let login = true;

    function handleLogin() {
        login = false;
    }
</script>

<main>
    {#if login}
        <TitleBar>
            <Logo />
            <Spacer />
            <ButtonClose />
        </TitleBar>

        <Content>
            <About />
            <Login {handleLogin} />
        </Content>
    {:else}
        <TitleBar>
            <Logo />
            <WelcomeMessage message="Welcome Kuqs, try out our new version!" />
            <Account />
            <ButtonClose />
        </TitleBar>

        <Content>
            <LaunchArea />
            <NewsContainer />
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