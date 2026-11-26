import {writable} from "svelte/store";
import {invoke} from "@tauri-apps/api/core";

// LiquidBounce's account list: {id, type, name, uuid, favorite, active}
export const accounts = writable([]);

export async function loadAccounts(options) {
    try {
        accounts.set(await invoke("list_accounts", {options}));
    } catch (error) {
        console.error("Failed to load accounts:", error);
    }
}
