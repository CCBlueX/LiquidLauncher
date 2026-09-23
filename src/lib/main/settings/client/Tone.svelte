<script>
    export let line;
</script>

<span class:strong={line.tone === "strong"}>{line.text}</span>

<style>
    .strong {
        color: white;
    }
</style>
