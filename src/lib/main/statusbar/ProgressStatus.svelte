<script>
    export let value;
    export let max;
    export let text;
    export let speed = 0;

    const UNITS = ["B/s", "KB/s", "MB/s", "GB/s"];

    function formatSpeed(bytesPerSecond) {
        let size = bytesPerSecond;
        let unit = 0;

        while (size >= 1024 && unit < UNITS.length - 1) {
            size /= 1024;
            unit++;
        }

        return `${size.toFixed(unit > 0 && size < 100 ? 1 : 0)} ${UNITS[unit]}`;
    }
</script>

<div class="wrapper">
    <div class="labels">
        <div class="text">{text}</div>
        {#if speed > 0}
            <div class="speed">{formatSpeed(speed)}</div>
        {/if}
    </div>
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

    .labels {
        color: white;
        position: absolute;
        top: 50%;
        left: 20px;
        right: 20px;
        transform: translateY(-50%);
        display: flex;
        align-items: center;
        gap: 16px;
    }

    .text {
        flex: 1;
        min-width: 0;
        text-overflow: ellipsis;
        overflow: hidden;
        white-space: nowrap;
    }

    .speed {
        flex: none;
        font-variant-numeric: tabular-nums;
    }
</style>
