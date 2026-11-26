<script>
    import {createEventDispatcher} from "svelte";
    import VerticalFlexWrapper from "../common/VerticalFlexWrapper.svelte";
    import TitleBar from "../common/TitleBar.svelte";
    import Logo from "../common/Logo.svelte";
    import ButtonClose from "../common/ButtonClose.svelte";
    import {openUrl} from "@tauri-apps/plugin-opener";
    import ButtonSetting from "../settings/ButtonSetting.svelte";
    import {_} from "svelte-i18n";

    const dispatch = createEventDispatcher();
    
    function handleAllowConnection() {
        dispatch("allowNonSecure");
    }
    
    function handleCancel() {
        dispatch("cancel");
    }
</script>

<VerticalFlexWrapper blur={false}>
    <TitleBar>
        <Logo />
        <ButtonClose />
    </TitleBar>

    <div class="warning-container">
        <h1>{$_("insecure.title")}</h1>
        <p class="warning-message">{$_("insecure.message")}</p>
        
        <div class="warning-details">
            <h3>{$_("insecure.risks")}</h3>
            <ul>
                <li>{$_("insecure.riskUnencrypted")}</li>
                <li>{$_("insecure.riskIntercepted")}</li>
                <li>{$_("insecure.riskCredentials")}</li>
            </ul>
            
            <h3 class="recommendation">{$_("insecure.recommendation")}</h3>
            <p>{$_("insecure.recommendationText")}</p>
        </div>

        <div class="action-buttons">
            <ButtonSetting text={$_("insecure.getWarp")} color="#4677ff" on:click={() => openUrl('https://1.1.1.1/')}></ButtonSetting>
            <ButtonSetting text={$_("insecure.allow")} color="#B83529" on:click={handleAllowConnection}></ButtonSetting>
            <ButtonSetting text={$_("common.cancel")} color="#707070" on:click={handleCancel}></ButtonSetting>
        </div>
    </div>
</VerticalFlexWrapper>

<style>
    .warning-container {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 2rem;
        max-width: 800px;
        margin: 0 auto;
        color: white;
        flex: 1;
    }

    h1 {
        color: #ff9800;
        font-size: 2.5rem;
        margin-bottom: 1rem;
    }

    .warning-message {
        font-size: 1.2rem;
        margin-bottom: 2rem;
        text-align: center;
    }

    .warning-details {
        background-color: rgba(0, 0, 0, 0.3);
        padding: 1.5rem;
        border-radius: 6px;
        width: 100%;
        margin-bottom: 2rem;
    }

    .warning-details h3 {
        margin-bottom: 0.75rem;
        color: #ff9800;
    }

    .warning-details ul {
        margin-left: 1.5rem;
        margin-bottom: 1.5rem;
    }

    .warning-details li {
        margin-bottom: 0.5rem;
    }

    .recommendation {
        margin-top: 1rem;
    }

    .action-buttons {
        display: flex;
        gap: 1rem;
        margin-top: 1rem;
    }
</style>
