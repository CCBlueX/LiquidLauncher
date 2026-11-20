<div class="wait" role="status">
    <span class="ring" aria-hidden="true"></span>
    <span>Waiting for sign-in...</span>
</div>

<style>
    .wait {
        display: flex;
        align-items: center;
        justify-content: center;
        column-gap: 12px;
        padding: 14px 0;
        color: white;
        font-size: 14px;
    }

    .ring {
        width: 20px;
        height: 20px;
        border: 3px solid white;
        border-color: white transparent transparent transparent;
        border-radius: 50%;
        animation: spin 1.2s cubic-bezier(.5, 0, .5, 1) infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }
</style>
