<script>
    import TitleBar from "../common/TitleBar.svelte";
    import Logo from "../common/Logo.svelte";
    import StatusBar from "./statusbar/StatusBar.svelte";
    import TextStatus from "./statusbar/TextStatus.svelte";
    import ProgressStatus from "./statusbar/ProgressStatus.svelte";
    import Account from "./Account.svelte";
    import ButtonClose from "../common/ButtonClose.svelte";

    export let account;
    export let accounts;
    export let running;
    export let progressState;
</script>

<TitleBar>
    <Logo />
    <StatusBar>
        {#if !running}
            <TextStatus
                    text="Welcome back, {account?.name}."
            />
        {:else}
            <ProgressStatus {...progressState} />
        {/if}
    </StatusBar>
    <Account
            {account}
            {accounts}
            on:showSettings
            on:manageAccounts
            on:switchAccount
    />
    <ButtonClose />
</TitleBar>
