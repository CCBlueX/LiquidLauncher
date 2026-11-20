<script>
    export let uuid;
    export let size = 48;
</script>

{#key uuid}
    <object data="https://avatar.liquidbounce.net/avatar/{uuid}" type="image/png" class="avatar" style="width: {size}px; height: {size}px" aria-label="avatar">
        <img src="img/steve.png" alt="avatar" class="avatar" style="width: {size}px; height: {size}px"/>
    </object>
{/key}

<style>
    .avatar {
        flex: none;
        display: block;
        border-radius: 50%;
        overflow: hidden;
        /* An <object> swallows clicks, the button around it has to get them. */
        pointer-events: none;
    }
</style>
