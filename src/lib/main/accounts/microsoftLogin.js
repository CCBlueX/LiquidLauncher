import {writable} from "svelte/store";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import {openUrl} from "@tauri-apps/plugin-opener";
import {writeText} from "@tauri-apps/plugin-clipboard-manager";

// status: "idle" | "webview" | "deviceCode" | "noMinecraft"
export function createMicrosoftLogin(getOptions, onAccount) {
    const status = writable("idle");

    // Rust cannot stop a sign-in. A cancelled one runs on and its result is ignored.
    let attempt = 0;
    // openUrl or writeText, waiting for the device code to arrive.
    let useLink = null;

    const stopListening = listen("microsoft_device_code", ({payload}) => {
        useLink?.(payload.directVerificationUri).catch(console.error);
        useLink = null;
    });

    function start(command, waitingStatus) {
        const current = ++attempt;
        status.set(waitingStatus);

        invoke(command, {options: getOptions()})
            .then((account) => {
                if (current === attempt) onAccount(account);
            })
            .catch((error) => {
                if (current === attempt) fail(error);
            });
    }

    function fail(error) {
        if (String(error) === "no_minecraft") {
            status.set("noMinecraft");
            return;
        }

        status.set("idle");
        alert("Microsoft authentication failed.\n\n" + error);
    }

    function startDeviceCode(linkAction) {
        useLink = linkAction;
        start("add_microsoft_device_code", "deviceCode");
    }

    function cancel() {
        attempt++;
        useLink = null;
        status.set("idle");
    }

    return {
        status,
        webview: () => start("add_microsoft_webview", "webview"),
        openLink: () => startDeviceCode(openUrl),
        copyLink: () => startDeviceCode(writeText),
        cancel,
        destroy() {
            cancel();
            stopListening.then((unlisten) => unlisten());
        },
    };
}
