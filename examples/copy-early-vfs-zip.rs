use std::fs::{create_dir_all, File};
use vfs_zip::*;
use vfs04::*;

fn main() {
    create_dir_all("target/tmp").unwrap();;
    let src = VfsPath::new(ZipReadOnly::new_strict(File::open("test/data/early-vfs-zip.zip").unwrap()).unwrap());
    let dst = VfsPath::new(ZipWriteOnly::new_weak(File::create("target/tmp/early-vfs-zip-copy.zip").unwrap()).unwrap());
    // NOTE: https://github.com/MaulingMonkey/vfs-zip/issues/1
    let copied = src.copy_dir(&dst.join("subdir").unwrap()).unwrap();
    assert_eq!(copied, 16);
}
