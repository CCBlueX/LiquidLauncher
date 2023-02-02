<script>
    export let text;
</script>

<div class="message">{text}</div>

<style>
    .message {
        font-family: monospace;
        font-size: 16px;
        color: white;
        user-select: text;
        word-break: normal;
        overflow-wrap: anywhere;
    }
</style>