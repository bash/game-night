use anyhow::{Error, Result};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

pub(crate) fn read_or_generate<F: GeneratedFile>(path: &Path, mut f: F) -> Result<F::Value> {
    let file_path = get_file_path(path, &f)?;
    create_dir_all(file_path.parent().unwrap())?;
    match write(&mut f, &file_path) {
        Err(e) if is_already_exists_error(&e) => read(&f, &file_path),
        result => result,
    }
}

pub(crate) trait GeneratedFile {
    type Value;

    fn generate(&mut self) -> Self::Value;

    fn file_name(&self) -> &'static str;

    fn write(&self, value: &Self::Value, write: &mut dyn Write) -> Result<()>;

    fn read(&self, read: &mut dyn Read) -> Result<Self::Value>;
}

fn get_file_path<F: GeneratedFile>(path: &Path, f: &F) -> Result<PathBuf> {
    let mut file_path = path.to_owned();
    file_path.push(f.file_name());
    Ok(file_path)
}

fn read<F: GeneratedFile>(f: &F, file_path: &Path) -> Result<F::Value> {
    f.read(&mut File::open(file_path)?)
}

fn write<F: GeneratedFile>(f: &mut F, file_path: &Path) -> Result<F::Value> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file_path)?;
    let value = f.generate();
    f.write(&value, &mut file)?;
    Ok(value)
}

fn is_already_exists_error(error: &Error) -> bool {
    underlying_io_error_kind(error) == Some(ErrorKind::AlreadyExists)
}

fn underlying_io_error_kind(error: &Error) -> Option<io::ErrorKind> {
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<io::Error>())
        .map(|e| e.kind())
        .next()
}
