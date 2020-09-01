#![cfg_attr(external_doc, feature(external_doc))]
#![cfg_attr(external_doc, doc(include = "../Readme.md"))]
#![cfg_attr(external_doc, warn(missing_docs))]
#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "vfs04"), allow(dead_code))]

mod error; pub use error::*;
#[path = "read/_read.rs"]   mod read;   pub use read::*;
#[path = "write/_write.rs"] mod write;  pub use write::*;
