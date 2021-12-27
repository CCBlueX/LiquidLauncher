<script>
    export let exit;
</script>

<button class="button" role="window-close" on:click={exit}></button>

<style>
    .button {
        height: 48px;
        width: 48px;
        background: unset;
        background-image: url("../img/icon/icon-button-close.svg");
        background-position: center;
        background-repeat: no-repeat;
        border: solid 1px rgba(255, 255, 255, .5);
        border-radius: 50%;
        vertical-align: middle;
        transition: background-color ease .2s;
    }

    .button:hover {
        background-color: rgba(255, 255, 255, .1);
    }
</style>