use anyhow::Result;
use assert_cmd::Command;
use assert_fs::prelude::*;
use std::{
    env::current_dir,
    fs,
    fs::DirEntry,
    io::Read,
    path::{Path, PathBuf},
};

fn cmd() -> Result<Command> {
    Ok(Command::cargo_bin("witme")?)
}

#[test]
fn help_case() -> Result<()> {
    let mut command = cmd()?;
    println!(
        "{:#?}",
        command
            .current_dir("./examples/counter")
            .arg("near")
            .arg("wit")
    );
    Ok(())
}

fn near_cmd(example: &Path) -> Result<Command> {
    let mut c = cmd()?;
    c.current_dir(example).arg("near");
    Ok(c)
}

fn wit_cmd(example: &Path) -> Result<Command> {
    let mut c = near_cmd(example)?;
    c.arg("wit");
    Ok(c)
}

fn test_example_wit(example: PathBuf) -> Result<()> {
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child("index.wit");
    (&mut wit_cmd(&example)?)
        .arg("-o")
        .arg(temp.join("index.wit"))
        .arg("--sdk")
        .unwrap();
    let mut f = fs::File::open(example.join("index.wit"))?;
    let mut actual_file = fs::File::open(&file)?;
    let mut contents = String::new();
    let mut actual = String::new();
    f.read_to_string(&mut contents)?;
    actual_file.read_to_string(&mut actual)?;
    assert_eq!(contents, actual);
    Ok(())
}

fn is_dir(d: Result<DirEntry, std::io::Error>) -> Option<PathBuf> {
    match d {
        Ok(entry) if entry.file_type().unwrap().is_dir() => Some(entry.path()),
        _ => None,
    }
}

#[test]
fn wit_tests() -> Result<()> {
    fs::read_dir(current_dir()?.join("examples"))?
        .filter_map(is_dir)
        .try_for_each(test_example_wit)
}
