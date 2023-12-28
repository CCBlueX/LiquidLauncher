<script>
    export let value;
    export let max;
    export let text;
</script>

<div class="wrapper">
    <div class="text">{text}</div>
    <progress class="progress" {value} {max} />
</div>

<style>
    .wrapper {
        position: relative;
    }

    .wrapper,
    .progress {
        width: 100%;
        height: 100%;
    }

    .progress::-webkit-progress-bar {
        background-color: transparent;
    }

    .progress::-webkit-progress-value {
        background-color: #4677ff;
    }

    .text {
        color: white;
        position: absolute;
        top: 50%;
        max-width: calc(100% - 40px);
        transform: translateY(-50%);
        left: 20px;
        text-overflow: ellipsis;
        overflow: hidden;
        white-space: nowrap;
    }
</style>
