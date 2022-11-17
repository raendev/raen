use anyhow::{Context, Result};
use cargo_metadata::Package;
use filetime::FileTime;
use std::{fs, path::Path};

pub trait PackageExt {
    fn witgen_dep(&self) -> bool;
}

impl PackageExt for Package {
    fn witgen_dep(&self) -> bool {
        self.metadata
            .as_object()
            .map_or(false, |metadata| metadata.contains_key("witgen"))
    }
}

pub fn get_time(path: &Path) -> Result<FileTime> {
    Ok(FileTime::from_last_modification_time(
        &fs::metadata(path).context(format!("failed to access {}", path.to_string_lossy()))?,
    ))
}

pub fn compress_file(p: &Path) -> Result<Vec<u8>> {
    let buf = fs::read(p)
        .map_err(anyhow::Error::from)
        .with_context(|| format!("{}", p.to_string_lossy()))?
        .into_boxed_slice();
    let mut out = Vec::<u8>::new();
    let params = brotli::enc::BrotliEncoderParams {
        quality: 11,
        ..Default::default()
    };

    brotli::BrotliCompress(&mut buf.as_ref(), &mut out, &params)?;
    Ok(out)
}
