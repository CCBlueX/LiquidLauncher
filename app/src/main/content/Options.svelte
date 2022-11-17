<script>
    export let options;
    export let logout;

    export let versionData;
    export let branches;
    export let builds;
    export let mods;

    export let updateBuilds;

    console.log(JSON.stringify(versionData));

    function storeOptions() {
        // store disabled mods into options
        options.disabledMods = mods.filter(x => !x.enabled).map(x => x.name);

        // store options to file
        Window.this.xcall("store_options", options);
    }

    function syncBuilds() {
        let branch = document.getElementById("branches").value;
        options.preferredBranch = branch;
        Window.this.xcall("store_options", options);

        updateBuilds(branch);
    }

    function syncVersionData() {
        let choosenBuild = document.getElementById("builds").value;

        let buildData;
        if (choosenBuild === "latest") {
            buildData = builds.find(x => x.release || options.showNightlyBuilds);
            options.preferredBuild = null;
        } else {
            let buildId = parseInt(choosenBuild);
            buildData = builds.find(x => buildId === x.buildId);
            options.preferredBuild = buildId;
        }

        if (buildData === undefined) {
            console.error("failed to sync version data to user selection", buildData);
            return;
        }
        Window.this.xcall("store_options", options);

        versionData = buildData;
    }

    // todo: fix hacky solution
    // due to https://sciter.com/forums/topic/select-options-is-undefined/
    setTimeout(function() {
        if (branches.includes(options.preferredBranch)) {
            document.getElementById("branches").value = options.preferredBranch;
        } else {
            document.getElementById("branches").value = branches[0];
        }

        if (builds.filter(x => x.release || options.showNightlyBuilds).some(x => x.buildId === options.preferredBuild)) {
            document.getElementById("builds").value = options.preferredBuild;
        } else {
            document.getElementById("builds").value = "Latest"
        }
    }, 1);
</script>

<div class="wrapper">
<h1>Options</h1>

<div class="inner_wrapper">
    <h2>Version Selection</h2>

    <select name="branches" id="branches" on:change={syncBuilds}>
        {#each branches as branch}
            <option value={branch}>{branch}</option>
        {/each}
    </select>
    <select name="builds" id="builds" on:change={syncVersionData}>
        {#if builds.filter(x => x.release || options.showNightlyBuilds).length > 0}
            <option value="latest">Latest</option>

            {#each builds as build}
                {#if build.release || options.showNightlyBuilds}
                    <option value={build.buildId}>{build.lbVersion + " git-" + build.commitId.substring(0, 7)}</option>
                {/if}
            {/each}
        {/if}
    </select>
    <br>
    <label>
        Show nightly builds
        <input type=checkbox bind:checked={options.showNightlyBuilds} on:change={syncBuilds}>
    </label>
</div>

<div class="inner_wrapper">
    <h2>Launcher options</h2>

    <label>
        Keep Launcher Open
        <input type=checkbox bind:checked={options.keepLauncherOpen} on:change={storeOptions}>
    </label>
    <br>
    <button class="logout" on:click={logout}>Logout</button>
<br>
</div>

<div class="inner_wrapper">
    <h2>Additional Mods</h2>
    <ul class="mods">
        {#each mods as mod}
            <li><input type=checkbox bind:checked={mod.enabled} disabled={mod.required} on:change={storeOptions}> {mod.name}</li>
        {/each}
    </ul>

<!--    <label>
        Memory Percentage ({options.memoryPercentage} %)
        <br>
        <input type=hslider min="20" max="100" bind:value={options.memoryPercentage} on:change={storeOptions}>
    </label>
    <br>
    <label>
        Custom JVM
        <br>
        <input type=text bind:value={options.customJavaPath} on:input={storeOptions}>
    </label>-->
</div>

</div>

<style>
    label, h1, h2, ul, li {
        color: white;
    }

    .wrapper {
        width: 1*;
        height: 1*;
        border-spacing: 24px;
        overflow: hidden;
    }

    .inner_wrapper {
        background-color: rgba(0, 0, 0, .36);
        padding: 24px;
        border-radius: 6px;
        height: 1*;
        overflow: scroll-indicator;
        border-spacing: 20px;
    }

    .scroll-indicator {
        height: 14px;
        background-image: url("/img/icon/icon-changelogs-scroll.svg");
        background-position: center;
        background-repeat: no-repeat;
    }

    .logout {
        margin-top: 0.5rem;
        margin-bottom: 0.5rem;
        background: unset;
        border: none;
        background-color: #f83939;
        color: white;
        font-size: 14px;
        border-radius: 6px;
        width: 0.1*;
        display: block;
        height: 2rem;
        transition: background-color ease .2s;
        font-family: "Gilroy",serif;
    }

    .logout:hover {
        background-color: #e23e4c;
    }

    .mods {
        list-style: none;
    }
</style>