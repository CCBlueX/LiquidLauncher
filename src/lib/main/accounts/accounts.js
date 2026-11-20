import {writable} from "svelte/store";
import {invoke} from "@tauri-apps/api/core";

export const MINECRAFT_STORE_URL = "https://www.minecraft.net/store/minecraft-java-bedrock-edition-pc";

// LiquidBounce's account list: {id, type, name, uuid, favorite}
export const accounts = writable([]);

export async function loadAccounts(options) {
    try {
        accounts.set(await invoke("list_accounts", {options}));
    } catch (error) {
        console.error("Failed to load accounts:", error);
    }
}

// options.json says "Premium" where the list says "Microsoft".
export function accountTypeName(account) {
    return account?.type === "Premium" ? "Microsoft" : account?.type;
}

export function isActiveAccount(entry, account) {
    return entry.type === accountTypeName(account) && entry.uuid === account?.id;
}

export function hasMicrosoftAccount(list) {
    return list.some((entry) => entry.type === "Microsoft");
}

// What is wrong with an offline username, or null.
export function usernameProblem(username) {
    if (username.length < 1 || username.length > 16) {
        return "Username must be between 1 and 16 characters long.";
    }
    if (!/^[a-zA-Z0-9_]+$/.test(username)) {
        return "Username can only contain letters, numbers, and underscores.";
    }
    return null;
}
