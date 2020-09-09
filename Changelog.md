## 0.2.1

*   Properly require zip 0.5.3 or higher (`is_dir`, `is_file`)
*   Workaround breaking change in zip 0.5.7 ([mvdnes/zip-rs#193])

[mvdnes/zip-rs#193]:  https://github.com/mvdnes/zip-rs/issues/193

## 0.2.0

*   Introduced vfs_zip::[ZipWriteOnly](https://docs.rs/vfs-zip/0.2.0/vfs_zip/struct.ZipWriteOnly.html)
*   Made `zip` features optional / re-exposed via crate features.
*   **Breaking:** This means that the following can no longer read compressed zip files:
    ```toml
    vfs = { version = "*", default-features = false }
    ```
    There should be no breaking changes with `default-features = true`.

## 0.1.0 (Initial Version)

* Introduced vfs_zip::[ZipReadOnly](https://docs.rs/vfs-zip/0.1.0/vfs_zip/struct.ZipReadOnly.html)
