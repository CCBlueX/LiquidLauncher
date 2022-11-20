<script>
    export let icon;
    export let text;
    export let url;

    function openUrl() {
        // Window.this.xcall("open", url);
    }
</script>

<button class="button" on:click={openUrl}>
    <div class="icon">
        <img src="img/icon/social/icon-{icon}.svg" alt={icon}>
    </div>
    <div class="text">{text}</div>
</button>

<style>
    .button {
        flow: horizontal;
        height: 28px;
        background: unset;
        border: none;
        border-radius: 4px;
        overflow: hidden;
    }

    .icon {
        width: 30px;
        height: 1*;
        position: relative;
        background-color: #4677FF;
    }

    .icon img {
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
    }

    .text {
        font-size: 12px;
        color: white;
        vertical-align: middle;
        padding: 0 10px;
        font-family: "Gilroy";
        background-color: rgba(0, 0, 0, .36);
        height: 1*;
        transition: background-color ease .2s;
    }

    .button:hover .text {
        background-color: #4677FF  
    }
</style>