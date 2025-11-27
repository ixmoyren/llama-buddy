use crate::{db, error::Whatever};
use rusqlite::Connection;
use std::{path::Path, sync::Arc};
use tokio::sync::Mutex;

pub(crate) mod init;
pub(crate) mod model;

pub(crate) fn connection_llama_buddy_db(
    path: impl AsRef<Path>,
) -> Result<Arc<Mutex<Connection>>, Whatever> {
    let conn = db::open_llama_buddy_db(path)?;
    let conn = Arc::new(Mutex::new(conn));
    Ok(conn)
}
