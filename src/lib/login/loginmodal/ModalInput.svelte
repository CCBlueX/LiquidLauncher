<script>
    export let icon;
    export let placeholder;
    export let value;

    export let characterLimit;
</script>

<div class="wrapper">
    <div class="icon">
        <img src="img/icon/icon-{icon}.svg" alt={icon}>
    </div>
    <input class="input" type="text" placeholder={placeholder} maxlength={characterLimit} bind:value>
</div>

<style>
    .wrapper {
        display: grid;
        grid-template-columns: max-content 1fr;
        background-color: rgba(0, 0, 0, .26);
        padding: 8px;
        border-radius: 6px;
        column-gap: 14px;
    }

    .icon {
        background-color: #4677FF;
        height: 38px;
        width: 38px;
        border-radius: 6px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .input {
        background-color: transparent;
        border: none;
        font-size: 16px;
        font-family: "Inter", sans-serif;
        color: white;
    }
</style>