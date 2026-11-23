import {spawnSync,execFileSync} from 'node:child_process';
import {existsSync,readFileSync,writeFileSync,cpSync} from 'node:fs';
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
const project=resolve(ios,'src-tauri/gen/apple'),spec=resolve(project,'project.yml');
cpSync(resolve(root,'src-tauri/icons/MusicOS.icon'),resolve(project,'MusicOS.icon'),{recursive:true});
let specification=readFileSync(spec,'utf8');
if(!specification.includes('path: MusicOS.icon'))specification=specification.replace('      - path: Assets.xcassets','      - path: MusicOS.icon\n        type: file\n      - path: Assets.xcassets');
if(!specification.includes('ASSETCATALOG_COMPILER_APPICON_NAME: MusicOS'))specification=specification.replace('        ENABLE_BITCODE: false','        ASSETCATALOG_COMPILER_APPICON_NAME: MusicOS\n        ENABLE_BITCODE: false');
if(!specification.includes('MusicOSSyncWidget:')){
 specification=specification.replace('    dependencies:\n      - framework: libapp.a','    dependencies:\n      - target: MusicOSSyncWidget\n        embed: true\n      - framework: libapp.a');
 specification+='\n  MusicOSSyncWidget:\n    type: app-extension\n    platform: iOS\n    deploymentTarget: "16.2"\n    sources:\n      - path: ../../../widget\n      - path: ../../../../assets/fonts/Figtree-Variable.ttf\n        buildPhase: resources\n    info:\n      path: MusicOSSyncWidget/Info.plist\n      properties:\n        CFBundleDisplayName: musicOS Sync\n        CFBundleShortVersionString: 0.1.2\n        CFBundleVersion: "0.1.2"\n        UIAppFonts: [Figtree-Variable.ttf]\n        NSExtension:\n          NSExtensionPointIdentifier: com.apple.widgetkit-extension\n    settings:\n      base:\n        PRODUCT_BUNDLE_IDENTIFIER: com.musicos.companion.sync-widget\n        PRODUCT_NAME: MusicOSSyncWidget\n        DEVELOPMENT_TEAM: '+(process.env.MUSICOS_APPLE_TEAM??'')+'\n        CODE_SIGN_STYLE: Automatic\n        APPLICATION_EXTENSION_API_ONLY: YES\n        SKIP_INSTALL: YES\n    dependencies:\n      - package: MusicOSActivityShared\n        product: MusicOSActivityShared\npackages:\n  MusicOSActivityShared:\n    path: ../../../activity-shared\n';
}
if(process.env.MUSICOS_APPLE_TEAM)specification=specification.replace(/DEVELOPMENT_TEAM:[^\n]*/g,'DEVELOPMENT_TEAM: '+process.env.MUSICOS_APPLE_TEAM);
if(readFileSync(spec,'utf8')!==specification){writeFileSync(spec,specification);const result=spawnSync('xcodegen',['generate','--spec',spec],{cwd:project,env,stdio:'inherit'});if(result.status!==0)process.exit(result.status??1);}
const pbx=resolve(project,'musicos-ios.xcodeproj/project.pbxproj'),beforeIcon=readFileSync(pbx,'utf8');
const iconType=beforeIcon.replace(/(\/\* MusicOS\.icon \*\/ = \{isa = PBXFileReference; )(?:lastKnownFileType|explicitFileType) = [^;]+;/g,'$1explicitFileType = com.apple.iconcomposer.icon; lastKnownFileType = folder.iconcomposer.icon;');
if(iconType!==beforeIcon)writeFileSync(pbx,iconType);
for(const name of ['project.yml','musicos-ios.xcodeproj/project.pbxproj']){
 const path=resolve(ios,'src-tauri/gen/apple',name),before=readFileSync(path,'utf8');
 const after=before.replace(/node tauri ios xcode-script/g,'node ../../../../node_modules/@tauri-apps/cli/tauri.js ios xcode-script');
 if(before!==after)writeFileSync(path,after);
}
run(['ios','build','--ci',...config,...process.argv.slice(2)]);
