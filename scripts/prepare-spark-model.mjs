import {readFileSync,existsSync,mkdirSync,copyFileSync,statSync,renameSync,rmSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {resolve,dirname,join} from 'node:path';
import {fileURLToPath} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const pinned=JSON.parse(readFileSync(join(root,'src-tauri/resources/spark-candidate.json'),'utf8'));
const source=process.env.MUSICOS_SPARK_MODEL_SOURCE||join(root,'out/rebuild-qa/model-evaluation-general/release-candidate',pinned.id);
if(!existsSync(join(source,'manifest.json')))throw Error('Verified Sparklist model is missing. Set MUSICOS_SPARK_MODEL_SOURCE to its staged folder before packaging.');
const manifest=JSON.parse(readFileSync(join(source,'manifest.json'),'utf8'));
if(JSON.stringify(manifest)!==JSON.stringify(pinned))throw Error('Sparklist bundle manifest differs from the pinned candidate.');
for(const file of pinned.files){if(!['audio.onnx','moods.json','preprocessor_config.json'].includes(file.name))throw Error('Unexpected model artifact');const path=join(source,file.name);if(statSync(path).size!==file.bytes||createHash('sha256').update(readFileSync(path)).digest('hex')!==file.sha256)throw Error(`Model checksum mismatch: ${file.name}`);}
if(!existsSync(join(source,'LICENSE.txt')))throw Error('The model license must accompany its bundle.');
const destination=join(root,'src-tauri/resources/models',pinned.id),staging=destination+'.staging';
rmSync(staging,{recursive:true,force:true});mkdirSync(staging,{recursive:true});
for(const name of ['manifest.json','LICENSE.txt',...pinned.files.map(file=>file.name)])copyFileSync(join(source,name),join(staging,name));
rmSync(destination,{recursive:true,force:true});renameSync(staging,destination);
console.log(`Bundled verified ${pinned.id}: ${pinned.files.reduce((sum,file)=>sum+file.bytes,0)} bytes.`);
