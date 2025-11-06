use crate::{db, db::CompletedStatus, error::Whatever};
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) async fn check_init_completed(conn: Arc<Mutex<Connection>>) -> Result<bool, Whatever> {
    let conn = conn.lock().await;
    db::check_init_completed(&conn)
}

pub(crate) async fn completed_init(
    conn: Arc<Mutex<Connection>>,
    completed_status: CompletedStatus,
) -> Result<(), Whatever> {
    let conn = conn.lock().await;
    db::completed_init(&conn, completed_status)
}
