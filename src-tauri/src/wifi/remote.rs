use super::{lock,Hub,HttpError};
use axum::{body::Bytes,extract::{Path,Query,State},http::{header,HeaderMap,StatusCode},response::IntoResponse,Json};
use musicos_desktop::{local::playback::{LocalPlaybackClient,LocalPlaybackSnapshot,RemotePlayback},models::Track};
use serde::Deserialize;
use serde_json::{json,Value};
use std::sync::Arc;
use std::time::{Duration,Instant};

#[derive(Clone)]
pub(super) struct RemoteHost {
    pub client:LocalPlaybackClient,
    pub library:Arc<dyn Fn()->anyhow::Result<crate::library::Library>+Send+Sync>,
}

pub(super) struct Receipt {
    peer:String,
    id:String,
    expires:Instant,
    result:Result<(),HttpError>,
}

fn allowed(hub:&Hub,headers:&HeaderMap)->Result<(String,RemoteHost),HttpError>{
    let id=hub.authorized(headers).ok_or((StatusCode::UNAUTHORIZED,"Pair this iPhone first"))?;
    ensure_permission(hub,&id)?;
    let host=hub.remote.clone().ok_or((StatusCode::NOT_FOUND,"Mac Remote is unavailable"))?;
    Ok((id,host))
}

fn ensure_permission(hub:&Hub,id:&str)->Result<(),HttpError>{
    if hub.running.load(std::sync::atomic::Ordering::Relaxed)&&lock(&hub.trust).peers.iter().any(|p|p.id==id&&p.remote_allowed){
        Ok(())
    }else{
        Err((StatusCode::FORBIDDEN,"Allow Mac Remote for this iPhone in musicOS Devices."))
    }
}

fn song(track:&Track)->Value{
    json!({"id":track.id,"title":track.title,"artist":track.artist,"album":track.album,
        "duration":track.duration,"has_cover":track.cover_art_path.is_some(),"available":track.available})
}

fn snapshot(hub:&Hub,player:LocalPlaybackSnapshot)->Value{
    let start=player.queue_index.map_or(0,|i|i+1);
    let queue=player.queue.iter().zip(player.queue_entry_ids.iter()).skip(start)
        .map(|(track,id)|json!({"entry":id,"track":song(track)})).collect::<Vec<_>>();
    json!({"version":1,"server_id":hub.identity.id,"name":hub.name,"queue_revision":player.queue_revision,
        "queue":queue,"track":player.track.as_ref().map(song),"entry":player.queue_index.and_then(|index|player.queue_entry_ids.get(index)),
        "is_playing":player.is_playing,"position":player.position,"speed":player.listening.speed})
}

pub(super) async fn state(State(hub):State<Arc<Hub>>,headers:HeaderMap)->Result<Json<Value>,HttpError>{
    let (_,host)=allowed(&hub,&headers)?;
    Ok(Json(snapshot(&hub,host.client.snapshot())))
}

#[derive(Deserialize)]
pub(super) struct LibraryQuery{q:Option<String>,offset:Option<usize>,album:Option<String>}

pub(super) async fn library(State(hub):State<Arc<Hub>>,headers:HeaderMap,Query(query):Query<LibraryQuery>)->Result<Json<Value>,HttpError>{
    let (peer,host)=allowed(&hub,&headers)?;
    let library=tokio::task::spawn_blocking(move||(host.library)()).await.ok().and_then(Result::ok)
        .ok_or((StatusCode::SERVICE_UNAVAILABLE,"The Mac library is unavailable"))?;
    ensure_permission(&hub,&peer)?;
    const PAGE:usize=60;
    let q=query.q.unwrap_or_default().chars().take(160).collect::<String>().to_lowercase();
    let offset=query.offset.unwrap_or_default();
    let album=query.album.as_ref().and_then(|id|library.albums.iter().find(|album|&album.id==id));
    let tracks=if let Some(album)=album{
        let index=library.tracks.iter().map(|track|(track.id,track)).collect::<std::collections::HashMap<_,_>>();
        album.track_ids.iter().filter_map(|id|index.get(id).copied()).collect::<Vec<_>>()
    }else{
        library.tracks.iter().filter(|track|q.is_empty()||format!("{} {} {}",track.title,track.artist,track.album).to_lowercase().contains(&q)).collect()
    };
    if query.album.is_some()&&album.is_none(){return Err((StatusCode::NOT_FOUND,"This album left the library"));}
    let albums=library.albums.iter().filter(|album|q.is_empty()||format!("{} {}",album.title,album.artist).to_lowercase().contains(&q)).collect::<Vec<_>>();
    let selected=album.map(|release|json!({"id":release.id,"title":release.title,"artist":release.artist,
        "track_ids":tracks.iter().filter(|track|track.available).take(1000).map(|track|track.id).collect::<Vec<_>>() }));
    Ok(Json(json!({"version":1,"selected":selected,"offset":offset,
        "tracks":tracks.iter().skip(offset).take(PAGE).map(|track|song(track)).collect::<Vec<_>>(),
        "albums":albums.iter().skip(offset).take(PAGE).map(|album|json!({"id":album.id,"title":album.title,"artist":album.artist,"count":album.track_ids.len()})).collect::<Vec<_>>(),
        "more":offset.saturating_add(PAGE)<tracks.len().max(if album.is_some(){0}else{albums.len()})})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command{request_id:String,action:RemotePlayback}

pub(super) async fn command(State(hub):State<Arc<Hub>>,headers:HeaderMap,Json(request):Json<Command>)->Result<Json<Value>,HttpError>{
    let (peer,host)=allowed(&hub,&headers)?;
    if !(16..=64).contains(&request.request_id.len())||!request.request_id.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-'){
        return Err((StatusCode::BAD_REQUEST,"Invalid command identifier"));
    }
    let hub=hub.clone();
    tokio::task::spawn_blocking(move||{
        let mut receipts=lock(&hub.remote_commands);
        ensure_permission(&hub,&peer)?;
        let now=Instant::now();
        receipts.retain(|receipt|receipt.expires>now);
        if let Some(receipt)=receipts.iter().find(|receipt|receipt.peer==peer&&receipt.id==request.request_id){
            receipt.result?;
            return Ok(Json(snapshot(&hub,host.client.snapshot())));
        }
        if receipts.len()>=512||receipts.iter().filter(|receipt|receipt.peer==peer&&receipt.expires.saturating_duration_since(now)>Duration::from_secs(59)).count()>=12{
            return Err((StatusCode::TOO_MANY_REQUESTS,"Wait for the Mac before sending more commands"));
        }
        let result=host.client.remote(request.action);
        let outcome=result.as_ref().map(|_|()).map_err(|_|(StatusCode::CONFLICT,"The Mac could not apply that command. Refresh and try again."));
        receipts.push(Receipt{peer,id:request.request_id,expires:now+Duration::from_secs(60),result:outcome});
        outcome?;
        Ok(Json(snapshot(&hub,result.expect("checked successful playback command"))))
    }).await.map_err(|_|(StatusCode::SERVICE_UNAVAILABLE,"The Mac could not complete that command"))?
}

pub(super) async fn cover(State(hub):State<Arc<Hub>>,headers:HeaderMap,Path(id):Path<i64>)->Result<impl IntoResponse,HttpError>{
    let (peer,host)=allowed(&hub,&headers)?;
    let bytes=tokio::task::spawn_blocking(move||->anyhow::Result<Vec<u8>>{
        use anyhow::Context;
        let library=(host.library)()?;
        let path=library.tracks.iter().find(|track|track.id==id).and_then(|track|track.cover_art_path.as_ref()).context("No cover")?;
        let mut reader=image::io::Reader::open(path)?.with_guessed_format()?;
        let mut limits=image::io::Limits::default();
        limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(128*1024*1024);
        reader.limits(limits);
        let mut output=std::io::Cursor::new(Vec::new());
        reader.decode()?.thumbnail(160,160).to_rgb8().write_to(&mut output,image::ImageOutputFormat::Jpeg(78))?;
        Ok(output.into_inner())
    }).await.ok().and_then(Result::ok).ok_or((StatusCode::NOT_FOUND,"Cover unavailable"))?;
    ensure_permission(&hub,&peer)?;
    Ok(([(header::CONTENT_TYPE,"image/jpeg"),(header::CACHE_CONTROL,"private, no-store")],Bytes::from(bytes)))
}
