<script>
    export let text;
</script>

<div class="text-status" data-tauri-drag-region>{text}</div>

<style>
    .text-status {
        color: white;
        align-items: stretch;
        display: flex;
        align-items: center;
        border-bottom: solid 2px #4677FF;
        width: max-content;
        margin: 0 auto;
    }
</style>