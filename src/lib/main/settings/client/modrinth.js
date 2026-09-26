// The options keep the mods installed from Modrinth by branch and Minecraft version.

function installed(options, build) {
    const branch = options.version.options[build.branch] ??= { modStates: {}, customModStates: {} };
    branch.modrinthMods ??= {};
    return branch.modrinthMods[build.mcVersion] ??= [];
}

export function track(options, build, entry) {
    const mods = installed(options, build);
    const index = mods.findIndex(mod => mod.projectId === entry.projectId);
    if (index === -1) {
        mods.push(entry);
    } else {
        mods[index] = entry;
    }
    return options.store();
}

export function untrack(options, build, projectId) {
    const mods = installed(options, build);
    const index = mods.findIndex(mod => mod.projectId === projectId);
    if (index !== -1) {
        mods.splice(index, 1);
    }
    return options.store();
}
