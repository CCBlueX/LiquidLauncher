<script>
    export let banner;
    export let title;
    export let text;
    export let targetURL;
    export let date;
</script>

<div class="news">
    <div class="preview-image" style="background-image: url('{banner.image}');">
        <div class="version">{banner.title}</div>
    </div>
    <div class="content">
        <div class="title-wrapper">
            <div class="title">{title}</div>
            <div class="date">{date}</div>
        </div>
        <div class="text">{text}</div>
        <button class="show-changelog">Read More</button>
    </div>
</div>

<style>
    .news {
        flow: horizontal;
        height: 170px;
        border-radius: 6px;
        overflow: hidden;
        width: 0.5*;
    }

    .preview-image {
        width: 130px;
        height: 1*;
        background-size: cover;
        position: relative;
        background-position: center;
    }

    .preview-image::before {
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

    .version {
        color: white;
        font-weight: 800;
        font-size: 34px;
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
    }

    .title-wrapper {
        flow: horizontal;
        border-spacing: 10px;
    }

    .date {
        font-size: 10px;
        color: rgba(255, 255, 255, .5);
        margin-left: 1*;
    }

    .title {
        color: white;
        font-weight: 800;
        font-size: 12px;
    }

    .content {
        background-color: rgba(0, 0, 0, .36);
        width: 1*;
        height: 1*;
        padding: 12px;
        border-spacing: 7px;
    }

    .text {
        font-size: 12px;
        color: rgba(255, 255, 255, .5);
        height: 1*;
        overflow: scroll-indicator;
    }

    .show-changelog {
        background: unset;
        border: none;
        background-color: #4677FF;
        color: white;
        font-size: 12px;
        border-radius: 4px;
        height: 30px;
        transition: background-color ease .2s;
        padding: 0 10px;
        font-family: "Gilroy";
    }

    .show-changelog:hover {
        background-color: #5884FF;
    }
</style>