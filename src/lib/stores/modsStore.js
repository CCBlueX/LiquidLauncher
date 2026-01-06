/**
 * Installed Mods Store
 * 
 * This store provides a centralized, reactive state for tracking installed mods.
 * Using a Svelte writable store instead of component-local state solves the
 * synchronization problem where multiple components need to reflect the same
 * installed mods list in real-time.
 * 
 * Why a store?
 * - When a mod is installed via ModrinthSearch, the UI needs to update immediately
 * - When a mod is deleted from VersionSelect, ModrinthSearch needs to reflect that
 * - Props and events create a one-way flow that requires manual refresh
 * - A store provides automatic reactivity across all subscribed components
 * 
 * Usage:
 * - Import and use $installedMods for reactive access
 * - Call setMods() when loading the mod list from backend
 * - Call addMod() after successful installation
 * - Call removeMod() after successful deletion
 */

import { writable } from 'svelte/store';

// Create the writable store with an empty array as initial value
const { subscribe, set, update } = writable([]);

export const installedMods = {
    subscribe,
    
    // Replace the entire mod list (used when loading from backend)
    setMods: (mods) => set(mods),
    
    // Add a newly installed mod to the list
    addMod: (mod) => update(mods => [...mods, mod]),
    
    // Remove a mod by name
    removeMod: (modName) => update(mods => 
        mods.filter(m => m.name !== modName)
    ),
    
    // Clear all mods
    clear: () => set([])
};
