<script>
    export let text;
</script>

<pre class="message">{text}</pre>

<style>
    .message {
        font-family: monospace;
        font-size: 16px;
        color: white;
        user-select: text;
        word-break: normal;
        white-space: pre-wrap;
        word-wrap: break-word;
    }
</style>