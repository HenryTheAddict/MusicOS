import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
import {existsSync} from 'node:fs';
const root=fileURLToPath(new URL('../../',import.meta.url));
function run(command,args){const r=spawnSync(command,args,{cwd:root,stdio:'inherit',env:process.env});if(r.status!==0)process.exit(r.status??1);}
run(process.execPath,[resolve(root,'node_modules/vite/bin/vite.js'),'build','--config','ios/vite.config.ts']);
const icons=resolve(root,'ios/src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset');
if(existsSync(icons))run('/usr/bin/swift',[resolve(root,'ios/scripts/prepare-icons.swift'),resolve(root,'src-tauri/icons/appicon.png'),icons]);
