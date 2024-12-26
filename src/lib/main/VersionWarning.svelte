<script>
    import { createEventDispatcher } from "svelte";
    import SettingsContainer from "../settings/SettingsContainer.svelte";
    import Description from "../settings/Description.svelte";
    import ButtonSetting from "../settings/ButtonSetting.svelte";

    export let launchVersionWarningCountdown;

    const dispatch = createEventDispatcher();
</script>

<SettingsContainer
        title="You are about to launch an unsupported version!"
        on:hideSettings={() => dispatch('hide')}
>
    <Description
            description="The selected version of LiquidBounce is no longer officially supported. We recommend upgrading to the latest version of LiquidBounce Nextgen, which works with all Minecraft versions from 1.7 onward."
    />
    <ButtonSetting
            text="Switch to Nextgen now"
            on:click={() => dispatch('switchToNextgen')}
            color="#4677FF"
    />
    <ButtonSetting
            disabled={launchVersionWarningCountdown > 0}
            text="Launch anyway{launchVersionWarningCountdown > 0 ? ` (${launchVersionWarningCountdown})` : ''}"
            on:click={() => dispatch('runClientAnyway')}
            color="#B83529"
    />
</SettingsContainer>