<script>
    export let title;
    export let placeholder;
    export let value;
</script>

<div class="text-setting">
    <div class="title">{title}</div>
    <input class="input" type="text" {placeholder} bind:value={value} />
</div>

<style>
    .title {
        color: white;
        margin-bottom: 5px;
    }

    .input {
        width: 100%;
        background-color: rgba(0, 0, 0, .26);
        border: none;
        border-bottom: solid 1px #4677FF;
        color: white;
        font-family: "Inter", sans-serif;
        padding: 5px;
        border-radius: 3px;
    }
</style>