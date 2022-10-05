use std::fs;
use std::path::Path;
use rust_embed::RustEmbed;
use anyhow::Result;

#[derive(RustEmbed)]
#[folder = "data/"]
struct Data;

fn main() -> Result<()> {
    for file_name in Data::iter() {
        println!("extracting {}", file_name.as_ref());

        if let Some(file) = Data::get(file_name.as_ref()) {
            let path = Path::new(file_name.as_ref());
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, file.data)?;
        }
    }
    Ok(())
}
