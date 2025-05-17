<script>
    import {createEventDispatcher} from "svelte";
    import SettingsContainer from "../settings/SettingsContainer.svelte";
    import Description from "../settings/Description.svelte";
    import ButtonSetting from "../settings/ButtonSetting.svelte";
    import {openUrl} from "@tauri-apps/plugin-opener";

    const dispatch = createEventDispatcher();
</script>

<SettingsContainer
        title="Welcome to LiquidBounce!"
        on:hideSettings={() => dispatch('hide')}
>
    <Description
            description="During the first launch or when a new update of LiquidBounce is available, a download window will show up. This is our way of generating advertising revenue for the client. If you do not want to see advertisements, you can purchase LiquidBounce Premium. You might also want to check out the quick start guide for tips on how to use LiquidBounce."
    />

    <ButtonSetting
            text="Quick Start Guide"
            on:click={() => openUrl("https://liquidbounce.net/docs/get-started")}
            color="#B83529"
    />

    <ButtonSetting
            text="Continue"
            on:click={() => dispatch('continue')}
            color="#4677FF"
    />
</SettingsContainer>