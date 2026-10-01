{
  lib,
  stdenv,
  rustPlatform,
  cargo-tauri,
  bun,
  nodejs,
  pkg-config,
  wrapGAppsHook3,
  writableTmpDirAsHomeHook,
  glib-networking,
  openssl,
  webkitgtk_4_1,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "liquidlauncher-unwrapped";
  version = (lib.importJSON ../package.json).version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.difference (lib.fileset.unions [
      ../index.html
      ../package.json
      ../bun.lock
      ../vite.config.js
      ../jsconfig.json
      ../src
      ../public
      ../src-tauri
      ../packaging/linux/net.ccbluex.liquidlauncher.metainfo.xml
    ]) (lib.fileset.maybeMissing ../src-tauri/target);
  };

  # Refresh outputHash whenever bun.lock changes
  node_modules = stdenv.mkDerivation {
    pname = "${finalAttrs.pname}-node_modules";
    inherit (finalAttrs) src version;

    nativeBuildInputs = [
      bun
      writableTmpDirAsHomeHook
    ];

    dontConfigure = true;
    dontFixup = true;

    buildPhase = ''
      runHook preBuild

      bun install \
        --cpu="*" \
        --frozen-lockfile \
        --ignore-scripts \
        --no-progress \
        --os="*"

      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p $out
      cp -r node_modules $out/node_modules

      runHook postInstall
    '';

    outputHash = "sha256-jjB2Sc5i6JLO2rtyoefrz3rd/zukO+2xlCi3p6cDFVc=";
    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
  };

  cargoRoot = "src-tauri";
  buildAndTestSubdir = finalAttrs.cargoRoot;
  cargoLock = {
    lockFile = ../src-tauri/Cargo.lock;
    allowBuiltinFetchGit = true;
  };

  postPatch = ''
    cp -r ${finalAttrs.node_modules}/node_modules .
    chmod -R +w node_modules
    patchShebangs --build node_modules
  '';

  # Updater artifacts need upstream's signing key
  tauriBuildFlags = [
    "--config"
    ''{"bundle":{"createUpdaterArtifacts":false}}''
  ];

  nativeBuildInputs = [
    bun
    cargo-tauri.hook
    nodejs
    pkg-config
    wrapGAppsHook3
  ];

  buildInputs = [
    glib-networking
    openssl
    webkitgtk_4_1
  ];

  meta = {
    description = "A custom Minecraft launcher for LiquidBounce";
    homepage = "https://liquidbounce.net";
    license = lib.licenses.gpl3Plus;
    mainProgram = "liquidlauncher";
    platforms = lib.platforms.linux;
  };
})
