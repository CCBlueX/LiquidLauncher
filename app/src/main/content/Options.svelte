<script>
    export let options;
    export let update;
    export let closeOptions;
    export let logout;

    function storeOptions() {
        Window.this.xcall("store_options", options);
        update();
    }

    console.log(JSON.stringify(options));
</script>

<br>

<label>
    Keep Launcher Open
    <input type=checkbox bind:checked={options.keepLauncherOpen} on:change={storeOptions}>
</label>
<br>
<label>
    Show nightly builds
    <input type=checkbox bind:checked={options.showNightlyBuilds} on:change={storeOptions}>
</label>

<br><br>


<button on:click={logout}>Log out from account</button>

<button class="button" on:click={closeOptions}>CLOSE</button>


<style>
    label {
        color: white;
    }
</style>