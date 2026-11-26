<script>
    import {createEventDispatcher} from "svelte";
    import SettingsContainer from "../settings/SettingsContainer.svelte";
    import Description from "../settings/Description.svelte";
    import ButtonSetting from "../settings/ButtonSetting.svelte";
    import {openUrl} from "@tauri-apps/plugin-opener";
    import {_} from "svelte-i18n";

    const dispatch = createEventDispatcher();
</script>

<SettingsContainer
        title={$_("firstRun.title")}
        on:hideSettings={() => dispatch('hide')}
>
    <Description
            description={$_("firstRun.description")}
    />

    <ButtonSetting
            text={$_("firstRun.quickStart")}
            on:click={() => openUrl("https://liquidbounce.net/docs/get-started")}
            color="#B83529"
    />

    <ButtonSetting
            text={$_("common.continue")}
            on:click={() => dispatch('continue')}
            color="#4677FF"
    />
</SettingsContainer>