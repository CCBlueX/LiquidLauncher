<script>
    import { fetchNews } from "../../utils/news";

    export let accountData;

    let mcVersion;
    let lbVersion;

    let title;
    let date;
    let text;
    let previewImage;

    (async () => {
        const news = (await fetchNews())[0];
        console.log(news);
        title = news.title;
        date = news.date;
        text = news.text;
        previewImage = news.banner.image;
    })();

    

    // Get all versions as struct
    const versionsHolder = {};
    Window.this.xcall("get_versions", versionsHolder);

    const versions = versionsHolder.versions;

    /* content of version:
        val.set_item("idx", idx as i32);
        val.set_item("liquidBounceVersion", &x.name);
        val.set_item("minecraftVersion", &x.mc_version);
        val.set_item("loaderName", &x.loader_version);
    */
    const latestVersion = versions[versions.length - 1];

    mcVersion = latestVersion.minecraftVersion;

    const versionContext = latestVersion.liquidBounceVersion.split("-");

    lbVersion = versionContext[0];
    const furtherVersionInfo = versionContext[1].split("+");

    const releaseType = furtherVersionInfo[0]; // nightly or release
    const commit = furtherVersionInfo[1]; // git commit of build

    console.log(furtherVersionInfo);

    // Handle play button to start client
    function handlePlay() {
        console.log("button clicked");

        // const label = document.getElementById('statusLabel');
        // const progressBar = document.getElementById('progress');

        function onProgress(action, value) {
            if (action === 'max') {
                // progressBar.max = value;
            } else if (action === 'progress') {
                // progressBar.value = value;
                console.log(progressBar.value + "/" + progressBar.max);
            } else if (action === 'label') {
                // label.textContent = value;

                console.log(value);
            }

            console.log(action + "/" + value);
        }

        function onOutput(type, value) {
            // let logArea = document.getElementById('log-area');

            // logArea.innerText += value;
            // logArea.scrollTop = logArea.scrollHeight;
        }

        function onDone() {
            // label.textContent = "Idle...";

            // startButton.disabled = false;
            // terminateButton.disabled = true;
        }
        function onError(error) {
            console.log("Error: " + error);

            // label.textContent = "Error: " + error;
        }

        // label.textContent = "Running...";

        Window.this.xcall("run_client", latestVersion.idx, accountData, onProgress, onOutput, onDone, onError);

        // startButton.disabled = true;
        // terminateButton.disabled = false;
    }

    // TODO: Implement version selection
    /*
    const versionSel = document.getElementById("select-div");

    let html = '';

    for (const x of asdf.versions) {
        html += '<option value="' + x.idx + '">' + x.minecraftVersion + " - " + x.liquidBounceVersion + '</option>';
    }

    versionSel.innerHTML = '<select id="version-selection">' + html + '</select>';
    */
</script>

<div class="wrapper">
    <div class="current-version" style="background-image: url('{previewImage}');">
        <div class="name">{title}</div>
        <div class="date">{date}</div>
    </div>

    <div class="changelog">
        <p>
            Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor 
        invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua.</p><br>
        
        <p>At vero eos et accusam 
        et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est 
        Lorem ipsum dolor sit amet. Lorem ipsum dolor sit amet, consetetur sadipscing elitr,</p><br> sed 
        diam nonumy eirmod tempor 
        <p>invidunt ut labore et dolore magna aliquyam erat, sed diam 
        voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd 
        gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor 
        sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut 
        labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam 
        et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus 
        est Lorem ipsum dolor sit amet.</p>
    </div>

    <div class="version-select">
        <div class="version">
            <img class="icon" src="img/icon/icon-version-lb.png" alt="liquidbounce">
            <div class="name">{lbVersion}</div>
            <div class="date">{releaseType}</div>
        </div>

        <div class="version">
            <img class="icon" src="img/icon/icon-version-mc.png" alt="minecraft">
            <div class="name">{mcVersion}</div>
            <div class="date">2021-05-07</div>
        </div>
    </div>

    <button class="launch" on:click={handlePlay}>Play</button>
</div>

<style>
    .wrapper {
        width: 293px;
        height: 1*;
        border-spacing: 17px;
        background-color: rgba(0, 0, 0, .36);
        padding: 24px;
        border-radius: 6px;
    }

    .current-version {
        height: 98px;
        width: 1*;
        overflow: hidden;
        border-radius: 6px;
        position: relative;
        background-size: cover;
        background-position: center;
    }

    .current-version::before {
        content: "";
        display: block;
        height: 1*;
        width: 1*;
        position: absolute;
        top: 0;
        left: 0;
        background: linear-gradient(top, rgba(0, 0, 0, .36), #4677ffc5);
        z-index: -1;
    }

    .current-version .name {
        z-index: 1000;
        font-weight: 800;
        color: white;
        font-size: 18px;
        max-width: 50%;
        position: absolute;
        top: 50%;
        transform: translate(15px, -50%);
    }

    .current-version .date {
        font-size: 12px;
        padding: 5px 12px;
        background-color: rgba(0, 0, 0, .68);
        color: white;
        border-radius: 4;
        position: absolute;
        top: 8px;
        right: 8px;
    }

    .changelog {
        font-size: 12px;
        color: rgba(255, 255, 255, .5);
        height: 1*;
        overflow: scroll-indicator;
    }

    .version-select {
        flow: horizontal;
        border-spacing: 8px;
    }

    .version-select .version {
        background-color: rgba(0, 0, 0, .36);
        border-radius: 6px;
        text-align: center;
        width: 1*;
        border-spacing: 2px;
        padding: 8px;
        border: solid 1px rgba(0, 0, 0, .36);
        transition: border-color ease .2s;
    }

    .version-select .version:hover {
        border-color: #4677ff;
    }

    .version-select .version .icon {
        margin-bottom: 4px;
    }

    .version-select .version .name {
        color: white;
        font-size: 12px;
    }

    .version-select .version .date {
        font-size: 10px;
        color: rgba(255, 255, 255, .5);
    }

    .launch {
        background: unset;
        border: none;
        background-color: #4677FF;
        color: white;
        font-size: 14px;
        border-radius: 6px;
        width: 1*;
        display: block;
        height: 50px;
        transition: background-color ease .2s;
        font-family: "Gilroy";
    }

    .launch:hover {
        background-color: #3E69E2;
    }
</style>