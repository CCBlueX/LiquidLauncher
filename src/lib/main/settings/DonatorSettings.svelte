<script>
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import SettingWrapper from "../../settings/SettingWrapper.svelte";
    import LiquidBounceAccount from "../../settings/LiquidBounceAccount.svelte";
    import Description from "../../settings/Description.svelte";
    import ButtonSetting from "../../settings/ButtonSetting.svelte";
    import {invoke} from "@tauri-apps/api/core";
    import {openUrl} from "@tauri-apps/plugin-opener";

    export let options;

    async function login() {
        try {
            const account = await invoke("client_account_authenticate");
            options.premium.account = account;
            await options.store();
        } catch (error) {
            console.error("Failed to authenticate client account:", error);
            alert(`Failed to authenticate client account: ${error}`);
        }
    }

    async function logout() {
        options.premium.account = null
    }
</script>

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
                description="There appears to be no premium associated with this account. Please link it on the account management page."
        />
    {/if}

    <ButtonSetting
            text="Manage Account"
            on:click={() => openUrl("https://user.liquidbounce.net")}
            color="#4677FF"
    />
    <ButtonSetting
            text="Logout"
            on:click={logout}
            color="#B83529"
    />
{:else}
    <Description
            description="By going premium, you not only support the ongoing development of the client but also receive a cape and the ability to bypass ads on the launcher."
    />

    <ButtonSetting
            text="Login with LiquidBounce Account"
            on:click={login}
            color="#4677FF"
    />
{/if}