use crate::config;
use anyhow::Result;
use turso::{Connection, Value};

pub async fn connect() -> Result<Connection> {
    let db_path = config::db_path();
    let db = turso::Builder::new_local(db_path.to_str().unwrap())
        .build()
        .await?;
    Ok(db.connect()?)
}

pub async fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS counter (
            id    INTEGER PRIMARY KEY,
            count TEXT NOT NULL
        )",
        (),
    )
    .await?;
    Ok(())
}

pub async fn read_counter(conn: &Connection) -> Result<Option<u128>> {
    let mut rows = conn
        .query("SELECT count FROM counter WHERE id = 1", ())
        .await?;

    if let Some(row) = rows.next().await? {
        let v = row.get_value(0)?;

        if let Value::Text(s) = v {
            let n = s.parse::<u128>()?;
            Ok(Some(n))
        } else {
            anyhow::bail!("expected TEXT for counter, got {:?}", v);
        }
    } else {
        Ok(None)
    }
}

pub async fn write_counter(conn: &Connection, counter: u128) -> Result<()> {
    let counter_str = counter.to_string();

    let rows_updated = conn
        .execute(
            "UPDATE counter SET count = ?1 WHERE id = 1",
            [counter_str.as_str()],
        )
        .await?;

    if rows_updated == 0 {
        conn.execute(
            "INSERT INTO counter (id, count) VALUES (1, ?1)",
            [counter_str.as_str()],
        )
        .await?;
    }

    Ok(())
}
