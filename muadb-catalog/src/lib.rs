mod types;
mod schema;
mod layout;
use muadb_util::*;

pub use types::*;
pub use schema::*;
pub use layout::*;

#[cfg(test)]
#[ctor::ctor]
fn init() {
    muadb_util::init_logging().unwrap();
}