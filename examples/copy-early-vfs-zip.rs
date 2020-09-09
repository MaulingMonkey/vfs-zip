use std::fs::{create_dir_all, File};
use vfs_zip::*;
use vfs04::*;

fn main() {
    create_dir_all("target/tmp").unwrap();
    let src = VfsPath::new(ZipReadOnly::new_strict(File::open("test/data/early-vfs-zip.zip").unwrap()).unwrap());
    let dst = VfsPath::new(ZipWriteOnly::new_weak(File::create("target/tmp/early-vfs-zip-copy.zip").unwrap()).unwrap());
    let copied = copy_dir_merge(&dst, &src).unwrap();
    assert_eq!(copied, 16);
}

/// NOTE: https://github.com/MaulingMonkey/vfs-zip/issues/1
fn copy_dir_merge(dst: &VfsPath, src: &VfsPath) -> VfsResult<usize> {
    let mut n = 0;

    if !src.exists() { return Err(VfsError::FileNotFound { path: src.as_str().into() }); }
    if !dst.exists() { dst.create_dir()?; n += 1; }

    for src in src.read_dir()? {
        let dst = dst.join(src.filename().as_str())?;
        match src.metadata()?.file_type {
            VfsFileType::Directory  => n += copy_dir_merge(&dst, &src)?,
            VfsFileType::File       => { src.copy_file(&dst)?; n += 1 },
        }
    }
    Ok(n)
}
