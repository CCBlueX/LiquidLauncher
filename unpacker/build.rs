extern crate embed_resource;

fn main() {
    embed_resource::compile("../liquidlauncher-manifest.rc");
    embed_resource::compile("../icons/icon_64x64.ico");
}