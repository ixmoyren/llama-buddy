use crate::{db, error::Whatever};
use rusqlite::Connection;
use std::{path::Path, sync::Arc};
use tokio::sync::Mutex;

pub mod init;
pub mod model;

pub(crate) fn connection(
    path: impl AsRef<Path>,
    db_name: impl AsRef<str>,
) -> Result<Arc<Mutex<Connection>>, Whatever> {
    let conn = db::open(path, db_name)?;
    let conn = Arc::new(Mutex::new(conn));
    Ok(conn)
}
