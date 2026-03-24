use tokio::fs::read_to_string;
use tokio_rusqlite::{Connection, Result};

pub async fn migrate(conn: &Connection) -> Result<()> {
    // TODO: Change the path to be win compatible
    let sql_stmt = read_to_string("src/db/migration-new.sql").await;
    conn.call(|conn| {
        let Ok(sql) = sql_stmt else {
            let e = sql_stmt.unwrap_err();
            panic!("couldnt extract migration statement. {e}");
        };

        conn.execute_batch(sql.as_str())?;
        Ok(())
    })
    .await?;
    Ok(())
}
