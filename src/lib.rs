#![cfg_attr(external_doc, feature(external_doc))]
#![cfg_attr(external_doc, doc(include = "../Readme.md"))]
#![cfg_attr(external_doc, warn(missing_docs))]

mod error; pub use error::*;
#[path = "read/_read.rs"] mod read;   pub use read::*;
