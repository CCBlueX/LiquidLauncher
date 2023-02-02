use anyhow::Result;
use sha1::{Sha1, Digest};
use std::path::PathBuf;

pub fn sha1sum(path: &PathBuf) -> Result<String> {
    // get sha1 of library file and check if it matches
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher)?;
    let hash = hasher.finalize();
    let hex_hash = base16ct::lower::encode_string(&hash);

    Ok(hex_hash)
}