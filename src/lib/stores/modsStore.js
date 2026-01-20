/**
 * Store for tracking installed mods across components.
 */

import { writable } from 'svelte/store';

const { subscribe, set, update } = writable([]);

export const installedMods = {
    subscribe,
    setMods: (mods) => set(mods),
    addMod: (mod) => update(mods => [...mods, mod]),
    removeMod: (modName) => update(mods => mods.filter(m => m.name !== modName)),
    clear: () => set([])
};
