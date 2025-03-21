<script>
    import {createEventDispatcher} from "svelte";
    import VerticalFlexWrapper from "../common/VerticalFlexWrapper.svelte";
    import TitleBar from "../common/TitleBar.svelte";
    import Logo from "../common/Logo.svelte";
    import ButtonClose from "../common/ButtonClose.svelte";
    import {openUrl} from "@tauri-apps/plugin-opener";
    import ButtonSetting from "../settings/ButtonSetting.svelte";

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
        <h1>Security Warning</h1>
        <p class="warning-message">
            LiquidLauncher is unable to connect to the secure (HTTPS) API endpoint, 
            but was able to establish a connection to a non-secure (HTTP) endpoint.
        </p>
        
        <div class="warning-details">
            <h3>Security Risks:</h3>
            <ul>
                <li>Your connection is not encrypted and may be visible to others on your network</li>
                <li>Data sent and received could be intercepted or modified by malicious actors</li>
                <li>Your login credentials could be compromised if transmitted over this connection</li>
            </ul>
            
            <h3 class="recommendation">Recommendation:</h3>
            <p>
                This issue is often caused by network restrictions or SSL certificate problems. 
                We recommend using a VPN such as Cloudflare WARP to establish a secure connection.
            </p>
        </div>

        <div class="action-buttons">
            <ButtonSetting text="Get Cloudflare WARP" color="#4677ff" on:click={() => openUrl('https://1.1.1.1/')}></ButtonSetting>
            <ButtonSetting text="Allow Non-Secure Connection" color="#B83529" on:click={handleAllowConnection}></ButtonSetting>
            <ButtonSetting text="Cancel" color="#707070" on:click={handleCancel}></ButtonSetting>
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
