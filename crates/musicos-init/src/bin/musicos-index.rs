use anyhow::Result;
use musicos_init::{database::Db, media};

fn main() -> Result<()> {
    let db_path =
        std::env::var("MUSICOS_MEDIA_DB").unwrap_or_else(|_| "out/desktop-media.sqlite3".into());
    let db = Db::open(&db_path)?;
    let indexed = media::index_default_locations(&db)?;
    println!("indexed_or_updated={indexed} db={db_path}");
    Ok(())
}
