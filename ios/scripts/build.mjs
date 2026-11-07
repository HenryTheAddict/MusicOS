import {spawnSync,execFileSync} from 'node:child_process';
import {existsSync,readFileSync,writeFileSync} from 'node:fs';
import {dirname,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
const ios=fileURLToPath(new URL('../',import.meta.url)),root=resolve(ios,'..');
const developer=process.env.DEVELOPER_DIR??'/Applications/Xcode.app/Contents/Developer';
const cargo=execFileSync('rustup',['which','cargo'],{encoding:'utf8'}).trim();
const env={...process.env,DEVELOPER_DIR:developer,PATH:`${dirname(cargo)}:${developer}/usr/bin:${process.env.PATH}`};
const cli=resolve(root,'node_modules/@tauri-apps/cli/tauri.js');
function run(args){const r=spawnSync(process.execPath,[cli,...args],{cwd:ios,env,stdio:'inherit'});if(r.status!==0)process.exit(r.status??1);}
const config=process.env.MUSICOS_APPLE_TEAM?['--config',JSON.stringify({bundle:{iOS:{developmentTeam:process.env.MUSICOS_APPLE_TEAM}}})]:[];
if(!existsSync(resolve(ios,'src-tauri/gen/apple/project.yml')))run(['ios','init','--ci','--skip-targets-install',...config]);
for(const name of ['project.yml','musicos-ios.xcodeproj/project.pbxproj']){
 const path=resolve(ios,'src-tauri/gen/apple',name),before=readFileSync(path,'utf8');
 const after=before.replace(/node tauri ios xcode-script/g,'node ../../../../node_modules/@tauri-apps/cli/tauri.js ios xcode-script');
 if(before!==after)writeFileSync(path,after);
}
run(['ios','build','--ci',...config,...process.argv.slice(2)]);
