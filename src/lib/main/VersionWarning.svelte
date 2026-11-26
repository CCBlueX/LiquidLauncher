<script>
    import { createEventDispatcher } from "svelte";
    import SettingsContainer from "../settings/SettingsContainer.svelte";
    import Description from "../settings/Description.svelte";
    import ButtonSetting from "../settings/ButtonSetting.svelte";
    import { _ } from "svelte-i18n";

    export let launchVersionWarningCountdown;

    const dispatch = createEventDispatcher();
</script>

<SettingsContainer
        title={$_("versionWarning.title")}
        on:hideSettings={() => dispatch('hide')}
>
    <Description
            description={$_("versionWarning.description")}
    />
    <ButtonSetting
            text={$_("versionWarning.switch")}
            on:click={() => dispatch('switchToLatest')}
            color="#4677FF"
    />
    <ButtonSetting
            disabled={launchVersionWarningCountdown > 0}
            text={launchVersionWarningCountdown > 0 ? $_("versionWarning.launchAnywayIn", { values: { seconds: launchVersionWarningCountdown } }) : $_("versionWarning.launchAnyway")}
            on:click={() => dispatch('runClientAnyway')}
            color="#B83529"
    />
</SettingsContainer>