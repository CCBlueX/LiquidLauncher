<script>
    import { createEventDispatcher } from "svelte";
    import ModalButton from "./ModalButton.svelte";
    import ModalInput from "./ModalInput.svelte";

    const dispatch = createEventDispatcher();

    function handleOfflineLoginClick(e) {
        dispatch("login");
    }

    function handleMicrosoftLoginClick(e) {

    }
</script>

<div class="modal">
    <div class="title">Log in</div>

    <ModalInput type="text" placeholder="Username" icon="person" />
    <ModalButton text="Offline login" primary={false} on:click={handleOfflineLoginClick} />
    <ModalButton text="Microsoft login" primary={true} on:click={handleMicrosoftLoginClick} />
</div>

<style>
    .modal {
        background-color: rgba(0, 0, 0, 0.26);
        padding: 30px;
        border-radius: 12px;
        width: 320px;
        display: flex;
        flex-direction: column;
        row-gap: 15px;
    }

    .title {
        color: white;
        font-size: 22px;
        margin: 0 auto;
        position: relative;
        width: max-content;
        margin-bottom: 40px;
    }

    .title::after {
        content: "";
        position: absolute;
        height: 5px;
        width: calc(100% - 10px);
        left: 50%;
        bottom: -20px;
        transform: translateX(-50%);
        background-color: #4677FF;
        border-radius: 5px;
    }
</style>