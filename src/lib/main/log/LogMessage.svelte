<script>
    export let line;
</script>

{#if line.kind === "text"}
    <div class="line">
        <pre class="message" class:frame={line.isFrame}>{line.text}</pre>
    </div>
{:else}
    <div class="line {line.level.toLowerCase()}">
        <span class="time">{line.time ?? ""}</span>
        <span class="level">{line.level}</span>
        {#if line.kind === "launcher"}
            <span class="source launcher">Launcher</span>
            <pre class="message">{line.text}</pre>
        {:else}
            <span class="source" title={line.thread}>{line.thread}</span>
            <pre class="message">{#if line.logger}<span class="logger">[{line.logger}] </span>{/if}{line.message}</pre>
        {/if}
    </div>
{/if}

<style>
    .line {
        display: grid;
        grid-template-columns: 8ch 5ch 14ch 1fr;
        column-gap: 1.5ch;
        padding: 1px 0;
        font-family: monospace;
        font-size: 13px;
        line-height: 1.45;
        color: white;
        user-select: text;
    }

    .time, .source, .logger {
        color: rgba(255, 255, 255, 0.45);
    }

    .source {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .source.launcher {
        color: #4677FF;
    }

    .level {
        font-weight: 600;
    }

    .message {
        grid-column: 4;
        font: inherit;
        white-space: pre-wrap;
        overflow-wrap: anywhere;
        tab-size: 4;
    }

    .message.frame {
        color: rgba(255, 255, 255, 0.45);
        word-break: break-all;
    }

    .info .level {
        color: #60b675;
    }

    .warn .level {
        color: #ff9800;
    }

    .error .level, .fatal .level, .error .message, .fatal .message {
        color: #ff4444;
    }

    .debug .level, .trace .level, .debug .message, .trace .message {
        color: rgba(255, 255, 255, 0.45);
    }
</style>
