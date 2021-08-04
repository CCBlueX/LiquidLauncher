<script>
    export let message;
</script>

<div class="message">
    <div class="content">{message}</div>
</div>

<style>
    .message {
        background-color: rgba(0, 0, 0, .36);
        color: white;
        height: 50px;
        font-size: 14px;
        width: 1*;
        border-radius: 6px;
        margin-right: 50px;
    }

    .content {
        height: 1*;
        vertical-align: middle;
        border-bottom: solid 2px #4677FF;
        width: max-content;
        margin: 0 auto;
    }
</style>