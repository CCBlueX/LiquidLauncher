<script>
    export let uuid;
    export let size;
</script>

{#key uuid}
    <object class="avatar" data="https://avatar.liquidbounce.net/avatar/{uuid}" type="image/png" style="--size: {size}px" aria-label="avatar">
        <img class="avatar" src="img/steve.png" alt="">
    </object>
{/key}

<style>
    .avatar {
        flex: none;
        display: block;
        width: var(--size);
        height: var(--size);
        border-radius: 50%;
        overflow: hidden;
        /* An <object> swallows clicks, the button around it has to get them. */
        pointer-events: none;
    }
</style>
