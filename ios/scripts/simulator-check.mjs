import {spawnSync} from 'node:child_process';
import {mkdirSync,readFileSync,writeFileSync,existsSync,copyFileSync} from 'node:fs';
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
 const path=resolve(fixtures,filename);if(mime==='audio/ogg'){const pcm=resolve(fixtures,`source${index}.wav`);run('/opt/homebrew/bin/ffmpeg',['-v','error','-y','-f','lavfi','-i',`sine=frequency=${220+index*110}:duration=5`,'-ac','2',pcm]);run('/opt/homebrew/bin/oggenc',['-Q','-b',index===1?'256':'320','-o',path,pcm]);}else run('/opt/homebrew/bin/ffmpeg',['-v','error','-y','-f','lavfi','-i',`sine=frequency=${220+index*110}:duration=5`,...codec,path]);
 const bytes=readFileSync(path),hash=createHash('sha256').update(bytes).digest('hex');
 const lyricPath=resolve(fixtures,`lyrics${index}.json`);writeFileSync(lyricPath,JSON.stringify({version:'1.0',metadata:{title:filename,artist:'musicOS tests',album:'Offline checks',offset_ms:0,instrumental:false,duration_ms:5000},plain:'Native playback\nWorks offline',lines:[{text:'Native playback',start_ms:0,end_ms:2400,words:[{text:'Native',start_ms:0,end_ms:1200},{text:'playback',start_ms:1200,end_ms:2400}]},{text:'Works offline',start_ms:2500,end_ms:4800}]}));
 tracks.push({id:index+1,path,title:filename,artist:'musicOS tests',album:'Offline checks',album_id:'test-release',genre:'Test',year:2026,track_number:index+1,disc_number:1,duration_ms:5000,sha256:hash,size:bytes.length,mime,artwork_path:cover,lyrics_path:lyricPath,favorited:false,server_id:'isolated-simulator-fixture'});
}
writeFileSync(resolve(library,'library.json'),JSON.stringify({version:1,tracks,collections:[],outbox:[],queue:[],queue_index:0}));
run('xcrun',['simctl','launch','--terminate-running-process',id,'com.musicos.companion'],{SIMCTL_CHILD_MUSICOS_COMPANION_SELF_TEST:'1'});
const report=resolve(library,'native-checks.json');let value;
for(let n=0;n<120;n++){if(existsSync(report)){value=JSON.parse(readFileSync(report));break;}await new Promise(r=>setTimeout(r,500));}
if(!value)throw Error('Native checks did not finish');console.log(JSON.stringify(value,null,2));
run('xcrun',['simctl','io',id,'screenshot','/tmp/musicos-phone-native-check.png']);
if(value.checks.some(c=>!c.passed))process.exitCode=1;
