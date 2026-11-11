import {spawnSync} from 'node:child_process';
import {createServer} from 'node:https';
import {tmpdir} from 'node:os';
import {mkdtempSync} from 'node:fs';
import {mkdirSync,readFileSync,writeFileSync,existsSync,copyFileSync,unlinkSync} from 'node:fs';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
const root=fileURLToPath(new URL('../../',import.meta.url)),id=process.argv[2];if(!id)throw Error('Pass an isolated simulator UDID');
const env={...process.env,DEVELOPER_DIR:process.env.DEVELOPER_DIR??'/Applications/Xcode.app/Contents/Developer'};
function run(command,args,extra={}){const r=spawnSync(command,args,{env:{...env,...extra},encoding:'utf8'});if(r.status!==0)throw Error(r.stderr||r.stdout);return r.stdout.trim();}
const app=resolve(root,'ios/src-tauri/gen/apple/build/arm64-sim/musicOS.app');
run('xcrun',['simctl','install',id,app]);
const data=run('xcrun',['simctl','get_app_container',id,'com.musicos.companion','data']);
const library=resolve(data,'Library/Application Support/musicOS'),fixtures=resolve(library,'test-fixtures');
mkdirSync(fixtures,{recursive:true});
if(existsSync(resolve(library,'library.json'))){const old=JSON.parse(readFileSync(resolve(library,'library.json')));if(old.tracks?.some(t=>t.server_id!=='isolated-simulator-fixture'||(!t.path.startsWith('test-fixtures/')&&!t.path.includes('/musicOS/test-fixtures/'))))throw Error('Refusing to replace a non-fixture simulator library');}
const tracks=[];const cover=resolve(fixtures,'cover.png');copyFileSync(resolve(root,'src-tauri/icons/appicon.png'),cover);
const variants=[['original.flac','audio/flac',['-c:a','flac']],['vorbis256.ogg','audio/ogg',['-c:a','libvorbis','-b:a','256k']],['vorbis320.ogg','audio/ogg',['-c:a','libvorbis','-b:a','320k']],['native.m4a','audio/mp4',['-c:a','aac','-b:a','192k']]];
for(const [index,[filename,mime,codec]] of variants.entries()){
 const path=resolve(fixtures,filename);if(mime==='audio/ogg'){const pcm=resolve(fixtures,`source${index}.wav`);run('/opt/homebrew/bin/ffmpeg',['-v','error','-y','-f','lavfi','-i',`sine=frequency=${220+index*110}:duration=12`,'-ac','2',pcm]);run('/opt/homebrew/bin/oggenc',['-Q','-b',index===1?'256':'320','-o',path,pcm]);}else run('/opt/homebrew/bin/ffmpeg',['-v','error','-y','-f','lavfi','-i',`sine=frequency=${220+index*110}:duration=12`,...codec,path]);
 const bytes=readFileSync(path),hash=createHash('sha256').update(bytes).digest('hex');
 const lyricPath=resolve(fixtures,`lyrics${index}.json`);writeFileSync(lyricPath,JSON.stringify({version:'1.0',metadata:{title:filename,artist:'musicOS tests',album:'Offline checks',offset_ms:0,instrumental:false,duration_ms:12000},plain:'Native playback\nWorks offline',lines:[{text:'Native playback',start_ms:0,end_ms:2400,words:[{text:'Native',start_ms:0,end_ms:1200},{text:'playback',start_ms:1200,end_ms:2400}]},{text:'Works offline',start_ms:2500,end_ms:4800}]}));
 tracks.push({id:index+1,path,title:filename,artist:'musicOS tests',album:'Offline checks',album_id:'test-release',genre:'Test',year:2026,track_number:index+1,disc_number:1,duration_ms:12000,sha256:hash,size:bytes.length,mime,artwork_path:cover,lyrics_path:lyricPath,favorited:false,server_id:'isolated-simulator-fixture'});
}
if(process.argv.includes('--long-labels')){
 for (const track of tracks) {
  track.title='AnUnbrokenSongTitle'.repeat(10);
  track.artist='A Very Long Artist Credit '.repeat(10).trim();
  track.album='AnUnbrokenAlbumTitle'.repeat(9);
 }
}
writeFileSync(resolve(library,'library.json'),JSON.stringify({version:1,tracks,collections:[],outbox:[],queue:[],queue_index:0}));
const report=resolve(library,'native-checks.json');if(existsSync(report))unlinkSync(report);
const tls=mkdtempSync(resolve(tmpdir(),'musicos-sync-test-'));
run('/usr/bin/openssl',['req','-x509','-newkey','rsa:2048','-nodes','-keyout',resolve(tls,'key.pem'),'-out',resolve(tls,'cert.pem'),'-days','1','-subj','/CN=127.0.0.1']);
const der=run('/usr/bin/openssl',['x509','-in',resolve(tls,'cert.pem'),'-outform','DER','-out',resolve(tls,'cert.der')]);
const pin=createHash('sha256').update(readFileSync(resolve(tls,'cert.der'))).digest('hex'),payload=readFileSync(cover),hash=createHash('sha256').update(payload).digest('hex');
const newSong={...tracks[0],id:9001};
delete newSong.path;
delete newSong.artwork_path;
delete newSong.lyrics_path;
const songPayload=readFileSync(tracks[0].path);
const manifest={version:1,server_id:'isolated-simulator-fixture',revision:'fixture-1',bytes_total:songPayload.length,tracks:[newSong],collections:[{id:'partial-sync-fixture',kind:'manual',name:'Partial transfer',full_track_ids:[9001,9002,9001]}]};
let audioRequests=0,syncRuns=0,preparationChecks=0;
const server=createServer({key:readFileSync(resolve(tls,'key.pem')),cert:readFileSync(resolve(tls,'cert.pem'))},(request,response)=>{
 if(request.url==='/v1/fixture'&&request.headers.authorization==='Bearer isolated-test'){
  response.writeHead(200,{'Content-Type':'application/octet-stream','Content-Length':payload.length,'ETag':`"${hash}"`});response.end(payload);return;
 }
 if(request.headers.authorization!=='Bearer fixture-client.fixture-token'){response.writeHead(403);response.end();return;}
 const json=value=>{response.writeHead(200,{'Content-Type':'application/json'});response.end(JSON.stringify(value));};
 switch(request.url){
  case '/v1/pair/status':syncRuns++;json({state:'approved'});break;
  case '/v1/preparation':json({preparing:syncRuns>=3||preparationChecks++===0,completed:0,total:1});break;
  case '/v1/manifest':json(manifest);break;
  case '/v1/sync-complete':json({state:'saved'});break;
  case '/v1/tracks/9001':
   audioRequests++;
   response.writeHead(200,{'Content-Type':newSong.mime,'Content-Length':songPayload.length,'ETag':`"${newSong.sha256}"`});response.end(songPayload);break;
  default:response.writeHead(404);response.end();
 }
});await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const port=server.address().port;
run('xcrun',['simctl','launch','--terminate-running-process',id,'com.musicos.companion'],{SIMCTL_CHILD_MUSICOS_COMPANION_SELF_TEST:'1',SIMCTL_CHILD_MUSICOS_TEST_SYNC_BASE:`https://127.0.0.1:${port}/v1`,SIMCTL_CHILD_MUSICOS_TEST_SYNC_PIN:pin,SIMCTL_CHILD_MUSICOS_TEST_SYNC_HASH:hash});
let value;
for(let n=0;n<120;n++){if(existsSync(report)){value=JSON.parse(readFileSync(report));break;}await new Promise(r=>setTimeout(r,500));}
server.close();if(!value)throw Error('Native checks did not finish');console.log(JSON.stringify(value,null,2));if(value.error||value.checks?.some(c=>!c.passed))throw Error('Native companion checks failed');if(audioRequests!==1)throw Error(`Expected one audio transfer across two syncs, got ${audioRequests}`);run('xcrun',['simctl','launch','--terminate-running-process',id,'com.musicos.companion']);
await new Promise(r=>setTimeout(r,1500));
run('xcrun',['simctl','io',id,'screenshot','/tmp/musicos-phone-native-check.png']);
if(value.checks.some(c=>!c.passed))process.exitCode=1;

server.close();
