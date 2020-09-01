[vfs]-[zip]: Virtual FileSystem abstractions for ZIP files

Currently this just bridges [vfs] and [zip].
Alternate VFS abstractions may be added in the future.
Caveats:

1.  [vfs] 0.4 lacks async interfaces, making it useless for browser targets.
2.  [zip] isn't amenable to re-entrant access.  This leads to Mutex spam, and
    forces open_file to copy/read the whole file up front.



<h2 name="features">Features</h2>

| Feature   | Description |
| --------- | ----------- |
| default   | 
| vfs04     | [vfs] = "[0.4.x](http://docs.rs/vfs/0.4)" interop




<h2 name="msrv">MSRV (Minimum Supported Rust Version)</h2>

Currently 1.34.0...ish.
*   [zip] 0.5.6 has a [MSRV of 1.34.0](https://github.com/mvdnes/zip-rs/blob/62dc406/README.md#msrv).
    However, zip's MSRV policy allows 0.5.7 to bump this, and `vfs-zip` does not pin zip to this version.
*   [vfs] 0.4.0 has a [MSRV of 1.32.0](https://github.com/manuel-woelker/rust-vfs/blob/c34f4ca/README.md#040-2020-08-13).
    However, it has no clear policy for when MSRV can be bumped.
*   Not all indirect dependencies have MSRV policies.  For example, I've already
    pinned flate2 to "<1.0.16" since "1.0.16" broke 1.34.0 with "extern crate alloc;"



<h2 name="license">License</h2>

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.



<h2 name="license">Contribution</h2>

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.



[vfs]:          https://lib.rs/crates/vfs
[zip]:          https://lib.rs/crates/zip
