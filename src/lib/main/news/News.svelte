<script>
    import { invoke } from "@tauri-apps/api/tauri";

    export let bannerText;
    export let title;
    export let date;
    export let bannerUrl;
    export let url;
    export let description;

    function handleClick() {
        invoke("open_url", { url: url });
    }
</script>

<div class="news">
    <div class="banner" style="background-image: linear-gradient(to bottom, transparent, #4677ffc5), url({bannerUrl});">
        <div class="banner-text">{bannerText}</div>
    </div>
    <div class="content">
        <div class="title-date">
            <div class="title">{title}</div>
            <div class="date">{new Date(date).toLocaleDateString()}</div>
        </div>
        <div class="description">{description}</div>
        <button class="button-show-news" type="button" on:click={handleClick}>Read more</button>
    </div>
</div>

<style>
    .news {
        height: 170px;
        display: grid;
        grid-template-columns: 2fr 3fr;
        border-radius: 12px;
        overflow: hidden;
    }

    .banner {
        background-size: cover;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .banner-text {
        color: white;
        font-weight: 600;
        font-size: 26px;
    }

    .content {
        background-color: rgba(0, 0, 0, .26);
        padding: 10px;
        display: flex;
        flex-direction: column;
        row-gap: 5px;
        flex: 1;
        column-gap: 5px;
        overflow: hidden;
    }

    .content .title {
        color: white;
        font-weight: 600;
        font-size: 12px;
        margin-right: 5px;
    }

    .content .date {
        font-size: 10px;
        color: rgba(255, 255, 255, .5);
        text-align: right;
    }

    .content .description {
        font-size: 12px;
        color: rgba(255, 255, 255, .5);
        overflow: auto;
        flex: 1;
    }

    .content .button-show-news {
        border: none;
        background-color: #4677FF;
        color: white;
        font-size: 12px;
        border-radius: 4px;
        height: 30px;
        transition: background-color ease .2s;
        padding: 0 10px;
        font-family: "Gilroy", sans-serif;
        cursor: pointer;
    }

    .content .button-show-news:hover {
        background-color: #3E69E2;
    }

    .content .title-date {
        display: grid;
        grid-template-columns: 1fr max-content;
    }
</style>