use anyhow::{ensure, Result};
use musicos_init::database::Db;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// One compressed display master per original revision. Full-size originals
/// remain canonical; thumbnails and palettes never repeatedly decode them.
pub fn rendition(path: &Path, pixels: u32) -> Result<PathBuf> {
    rendition_in(path, pixels, &musicos_desktop::storage::desktop_data_dir().join("artwork-renditions-v2"))
}

fn rendition_in(path: &Path, pixels: u32, directory: &Path) -> Result<PathBuf> {
    ensure!([128, 640, 1200, 1280].contains(&pixels), "Unsupported artwork size");
    // Legacy 1280 requests now use the same 1200 px master.
    let pixels = pixels.min(1200);
    let path = path.canonicalize()?;
    let meta = path.metadata()?;
    let identity = format!("jpeg90-png-v3-oriented:{}:{}:{:?}", path.display(), meta.len(), meta.modified()?);
    let stem = |size| directory.join(format!("{:x}-{size}", Sha256::digest(identity.as_bytes())));
    let cached = |size| {
        ["jpg", "png"].into_iter().map(|ext| stem(size).with_extension(ext)).find(|candidate| {
            if !candidate.is_file() { return false; }
            match image::image_dimensions(candidate) {
                Ok((w, h)) if w > 0 && h > 0 && w.max(h) <= size => true,
                _ => { let _ = std::fs::remove_file(candidate); false }
            }
        })
    };
    if let Some(target) = cached(pixels) { return Ok(target); }
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(target) = cached(pixels) { return Ok(target); }
    std::fs::create_dir_all(directory)?;
    let master = match cached(1200) {
        Some(master) => master,
        None => {
            let (w, h) = image::image_dimensions(&path)?;
            ensure!(w as u64 * h as u64 <= 36_000_000, "Artwork exceeds the decoding limit");
            #[cfg(target_os = "macos")]
            let original = {
                // ImageIO downsamples during decoding and applies EXIF rotation.
                // Rust only opens this bounded temporary, never another full copy.
                let prepared = stem(1200).with_extension(format!("prepared-{}.png", std::process::id()));
                let result = (|| -> Result<image::DynamicImage> {
                    musicos_desktop::local::decoded_audio::apple::artwork_png(&path, &prepared, w.max(h).min(1200))?;
                    Ok(image::open(&prepared)?)
                })();
                let _ = std::fs::remove_file(prepared);
                result?
            };
            #[cfg(not(target_os = "macos"))]
            let original = {
                let mut reader = image::io::Reader::open(&path)?.with_guessed_format()?;
                let mut limits = image::io::Limits::default();
                limits.max_alloc = Some(192 * 1024 * 1024);
                reader.limits(limits);
                reader.decode()?
            };
            let master = if original.width().max(original.height()) > 1200 {
                original.resize(1200, 1200, image::imageops::FilterType::Lanczos3)
            } else { original };
            publish(&master, &stem(1200))?
        }
    };
    if pixels == 1200 { return Ok(master); }
    let image = image::open(&master)?;
    if image.width().max(image.height()) <= pixels { return Ok(master); }
    let thumbnail = image.resize(pixels, pixels, image::imageops::FilterType::Lanczos3);
    publish(&thumbnail, &stem(pixels))
}

fn publish(image: &image::DynamicImage, stem: &Path) -> Result<PathBuf> {
    // Many embedded PNGs have an alpha channel filled entirely with 255.
    // Preserve actual transparency, without storing opaque photos as huge PNGs.
    let transparent = image.color().has_alpha() && image.to_rgba8().pixels().any(|pixel| pixel[3] != 255);
    let target = stem.with_extension(if transparent { "png" } else { "jpg" });
    let temporary = target.with_extension(format!("tmp-{}", std::process::id()));
    let result = (|| -> Result<()> {
        let mut file = std::io::BufWriter::new(std::fs::File::create(&temporary)?);
        if transparent {
            image.write_to(&mut file, image::ImageOutputFormat::Png)?;
        } else {
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 90).encode_image(&image.to_rgb8())?;
        }
        use std::io::Write;
        file.flush()?;
        file.get_ref().sync_all()?;
        std::fs::rename(&temporary, &target)?;
        Ok(())
    })();
    if result.is_err() { let _ = std::fs::remove_file(&temporary); }
    result?;
    Ok(target)
}
/// Upgrade only an old generated thumbnail whose embedded original matches it.
pub fn original(database:&Path,source:&Path)->Result<PathBuf>{
 // Ordinary renditions never need a database connection. Legacy upgrades are
 // serialized before opening SQLite, keeping first paint bounded.
 if !source.starts_with(database.parent().unwrap_or(Path::new(".")).join("cover-art/v2")){return Ok(source.into());}
 static UPGRADE:std::sync::Mutex<()>=std::sync::Mutex::new(());let _upgrade=UPGRADE.lock().unwrap_or_else(|e|e.into_inner());
 let _db=Db::open(&database.to_string_lossy())?;use rusqlite::OptionalExtension;let c=musicos_init::database::Connection::open(database)?;let row=c.query_row("SELECT t.id,t.path,t.modified_at FROM tracks t LEFT JOIN track_metadata_overrides o ON o.track_id=t.id WHERE t.cover_art_path=? AND o.artwork_path IS NULL LIMIT 1",[source.to_string_lossy()],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?))).optional()?;let Some((id,file,modified))=row else{return Ok(source.into())};let Some(full)=crate::desktop_metadata::song_artwork(database,id,Path::new(&file))? else{return Ok(source.into())};let(old_w,old_h)=image::image_dimensions(source)?;let(w,h)=image::image_dimensions(&full)?;if w.max(h)<=old_w.max(old_h){return Ok(source.into());}let small=|path:&Path|->Result<_>{Ok(image::open(path)?.resize_to_fill(24,24,image::imageops::FilterType::Triangle).to_rgb8())};let old=small(source)?;let new=small(&full)?;let error=old.as_raw().iter().zip(new.as_raw()).map(|(a,b)|(*a as f32-*b as f32).abs()).sum::<f32>()/old.as_raw().len() as f32;if error<=6. {
 publish_original(&c,source,&full,id,modified)?;return Ok(full);
 }Ok(source.into())}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_framing_alpha_and_never_upscales() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("original.png");
        image::RgbaImage::new(70, 120).save(&source).unwrap();
        let cache = temp.path().join("cache");
        let result = rendition_in(&source, 128, &cache).unwrap();
        assert_eq!(image::image_dimensions(&result).unwrap(), (70, 120));
        assert_eq!(result.extension().unwrap(), "png");
        assert_eq!(image::open(result).unwrap().to_rgba8().get_pixel(0, 0)[3], 0);
        assert!(rendition_in(&source, 3000, &cache).is_err());
    }
    #[test]
    fn compressed_master_is_shared_by_thumbnails_and_legacy_requests() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("photo.png");
        image::RgbaImage::from_fn(1800, 2400, |x,y| image::Rgba([(x%255) as u8,(y%255) as u8,120,255])).save(&source).unwrap();
        let original = std::fs::read(&source).unwrap();
        let cache = temp.path().join("cache");
        let small = rendition_in(&source, 128, &cache).unwrap();
        let master = rendition_in(&source, 1200, &cache).unwrap();
        let modified = master.metadata().unwrap().modified().unwrap();
        let grid = rendition_in(&source, 640, &cache).unwrap();
        assert_eq!(image::image_dimensions(small).unwrap(), (96, 128));
        assert_eq!(image::image_dimensions(grid).unwrap(), (480, 640));
        assert_eq!(image::image_dimensions(&master).unwrap(), (900, 1200));
        assert_eq!(master.extension().unwrap(), "jpg");
        assert_eq!(rendition_in(&source, 1280, &cache).unwrap(), master);
        assert_eq!(master.metadata().unwrap().modified().unwrap(), modified);
        assert_eq!(std::fs::read(source).unwrap(), original);
        assert_eq!(std::fs::read_dir(cache).unwrap().count(), 3);
    }
    #[test]
    fn concurrent_requests_publish_once_and_corrupt_cache_recovers() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("photo.png");
        image::RgbImage::new(1300, 1400).save(&source).unwrap();
        let cache = temp.path().join("cache");
        std::thread::scope(|scope| {
            let jobs: Vec<_> = [128,640,1200,1280].into_iter().map(|size| {
                let source=&source; let cache=&cache;
                scope.spawn(move || rendition_in(source,size,cache).unwrap())
            }).collect();
            for job in jobs { assert!(job.join().unwrap().is_file()); }
        });
        let master = rendition_in(&source,1200,&cache).unwrap();
        std::fs::write(&master,b"broken").unwrap();
        assert_eq!(rendition_in(&source,1200,&cache).unwrap(),master);
        let (w,h)=image::image_dimensions(master).unwrap();assert_eq!(h,1200);assert!((1114..=1115).contains(&w));
        assert_eq!(std::fs::read_dir(cache).unwrap().count(),3);
    }
    #[cfg(target_os="macos")]
    #[test]
    fn display_cache_applies_exif_orientation() {
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("rotated.jpg");
        let mut encoded=Vec::new();image::codecs::jpeg::JpegEncoder::new(&mut encoded).encode_image(&image::RgbImage::new(80,120)).unwrap();
        let exif=b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
        let mut jpeg=vec![0xff,0xd8,0xff,0xe1];jpeg.extend_from_slice(&((exif.len()+2)as u16).to_be_bytes());jpeg.extend_from_slice(exif);jpeg.extend_from_slice(&encoded[2..]);std::fs::write(&source,jpeg).unwrap();
        let display=rendition_in(&source,1200,&temp.path().join("cache")).unwrap();assert_eq!(image::image_dimensions(display).unwrap(),(120,80));
    }
    #[test]
    fn changed_original_gets_a_new_revision() {
        let temp = tempfile::tempdir().unwrap(); let source=temp.path().join("photo.png"); let cache=temp.path().join("cache");
        image::RgbImage::new(40,60).save(&source).unwrap(); let first=rendition_in(&source,1200,&cache).unwrap();
        image::RgbImage::new(80,90).save(&source).unwrap(); let next=rendition_in(&source,1200,&cache).unwrap();
        assert_ne!(first,next); assert_eq!(image::image_dimensions(next).unwrap(),(80,90));
    }
}

fn publish_original(c:&musicos_init::database::Connection,source:&Path,full:&Path,id:i64,modified:i64)->Result<()>{
 let tx=c.unchecked_transaction()?;
 tx.execute("UPDATE tracks SET cover_art_path=?1,cover_art_source_modified_at=?2 WHERE id=?3",rusqlite::params![full.to_string_lossy(),modified,id])?;
 tx.execute("UPDATE tracks SET cover_art_path=?1 WHERE cover_art_path=?2",rusqlite::params![full.to_string_lossy(),source.to_string_lossy()])?;
 let has_canonical:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='desktop_album_artwork_v1')",[],|r|r.get(0))?;
 if has_canonical{tx.execute("UPDATE desktop_album_artwork_v1 SET path=?1 WHERE path=?2",rusqlite::params![full.to_string_lossy(),source.to_string_lossy()])?;}
 tx.commit()?;Ok(())}
#[cfg(test)]mod upgrade_tests{use super::*;
 #[test]fn ordinary_images_do_not_open_a_database(){let temp=tempfile::tempdir().unwrap();let image=temp.path().join("manual.png");std::fs::write(&image,b"fixture").unwrap();let absent=temp.path().join("missing/db.sqlite");assert_eq!(original(&absent,&image).unwrap(),image);assert!(!absent.exists());}
 #[test]fn verified_upgrade_updates_canonical_references_once(){let c=musicos_init::database::Connection::open_in_memory().unwrap();c.execute_batch("CREATE TABLE tracks(id INTEGER PRIMARY KEY,cover_art_path TEXT,cover_art_source_modified_at INTEGER); CREATE TABLE desktop_album_artwork_v1(album_id TEXT PRIMARY KEY,path TEXT); INSERT INTO tracks VALUES(1,'old.png',1),(2,'old.png',1),(3,'different.png',1); INSERT INTO desktop_album_artwork_v1 VALUES('edition-a','old.png'),('edition-b','different.png')").unwrap();publish_original(&c,Path::new("old.png"),Path::new("full.png"),1,2).unwrap();assert_eq!(c.query_row("SELECT count(*) FROM tracks WHERE cover_art_path='full.png'",[],|r|r.get::<_,i64>(0)).unwrap(),2);assert_eq!(c.query_row("SELECT path FROM desktop_album_artwork_v1 WHERE album_id='edition-a'",[],|r|r.get::<_,String>(0)).unwrap(),"full.png");assert_eq!(c.query_row("SELECT path FROM desktop_album_artwork_v1 WHERE album_id='edition-b'",[],|r|r.get::<_,String>(0)).unwrap(),"different.png");}
}
