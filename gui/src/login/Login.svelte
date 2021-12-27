<script>
    export let handleMojangLogin;
    export let handleMicrosoftLogin;
    export let handleOfflineLogin;

    let username = ""
    let password = ""

    let passwordShown = false;

    function handleRevealPassword() {
        passwordShown = !passwordShown;
    }

    function clickMojangLogin() {
        handleMojangLogin(username, password);
    }

    function clickMicrosoftLogin() {
        handleMicrosoftLogin();
    }

    function clickOfflineLogin() {
        handleOfflineLogin(username);
    }
</script>

<div class="login">
    <div class="title">Log in</div>
    <div class="divider"></div>
    <div class="input-wrapper">
        <div class="icon">
            <img src="img/icon/icon-person.svg" alt="icon">
        </div>
        <input class="input-text" type="text" bind:value={username} placeholder="Username or e-mail address">
    </div>
    <div class="input-wrapper">
        <div class="icon">
            <img src="img/icon/icon-lock.svg" alt="icon">
        </div>
        <!-- type="{passwordShown ? "text" : "password"}" senkus pls fix -->
        <input class="input-text" bind:value={password} placeholder="Password">
        <img class="button-reveal-password" on:click={handleRevealPassword} src="img/icon/icon-eye.svg" alt="reveal">
    </div>
    <div class="button-large primary" on:click={clickMojangLogin}>Login</div>
    <div class="button-large" on:click={clickMicrosoftLogin}>Microsoft Login</div>
    <div class="button-large" on:click={clickOfflineLogin} >Use as Offline Account</div>
</div>

<style>
    .login {
        background-color: rgba(0, 0, 0, .68);
        width: 320px;
        text-align: center;
        border-radius: 12px;
        padding: 25px 25px 10px 25px;
        margin: 0 50px;
    }

    .title {
        color: white;
        font-size: 22px;
        font-weight: 400;
        margin-bottom: 15px;
    }

    .divider {
        background-color: #4677FF;
        height: 5px;
        width: 36px;
        border-radius: 3px;
        margin: 0 auto;
        margin-bottom: 20px;
    }

    .button-reveal-password {
        vertical-align: middle;
        margin-right: 20px;
        behavior: button;
    }

    .input-wrapper {
        background-color: rgba(0, 0, 0, .36);
        border-radius: 6px;
        flow: horizontal;
        margin: 12px 0;
        position: relative;
        border: solid 1px rgba(0, 0, 0, .36);
    }

    .input-wrapper.failure {
        border-color: #FC4130;
    }

    .input-text {
        width: 1*;
        vertical-align: middle;
        margin: 0 8px 0 12px;
        font-family: "Gilroy";
        color: white;
        font-weight: 400;
        font-size: 12px;
        background: transparent;
        border: none;
    }

    .input-text:empty {
        color: rgba(255, 255, 255, .5);
    }

    .icon {
        width: 38px;
        height: 38px;
        background-color: #4677FF;
        vertical-align: middle;
        border-radius: 4px;
        margin: 8px;
    }

    .input-wrapper.failure .icon {
        background-color: #FC4130;
    }

    .button-large {
        display: block;
        width: 1*;
        behavior: button;
        margin: 15px 30px;
        font-family: "Gilroy";
        color: white;
        font-size: 12px;
        font-weight: 400;
        background: unset;
        background-color: rgba(0, 0, 0, .36);
        border-radius: 6px;
        border: none;
        padding: 20px 0;
        transition: background-color ease .2s;
    }

    .button-large:hover {
        background-color: rgba(0, 0, 0, .5);
    }

    .button-large.primary {
        margin-top: 20px;
        background-color: #4677FF;
    }

    .button-large.primary:hover {
        background-color: #3E69E2;
    }
</style>