use crate::db::insert_config;
use rusqlite::Connection;

pub(crate) fn save_library_to_config(html: String, conn: &Connection) {
    let html = html.as_bytes();
    let html_sha256 = http_extra::sha256::digest(html);
    insert_config(
        conn,
        "model_library_html_digest".to_owned(),
        html_sha256.as_bytes().to_vec(),
    )
    .unwrap();
    insert_config(&conn, "model_library_html_data".to_owned(), html.to_vec()).unwrap();
}
