#!/usr/bin/env node
// Explicit, reversible personal theme installation. No plugins or credentials change.
import {readFile,mkdir,writeFile,copyFile,rename,chmod} from 'node:fs/promises';
import {homedir} from 'node:os';import {join,dirname} from 'node:path';import {fileURLToPath} from 'node:url';
const root=join(dirname(fileURLToPath(import.meta.url)),'..'),profile=join(homedir(),'Library/Application Support/vesktop'),destination=join(profile,'settings/quickCss.css');
if(!process.argv.includes('--apply')){console.log('MusicOS theme for Vesktop: local Figtree font, classic MusicOS blue surfaces, blue controls. Run with --apply to back up and replace Quick CSS.');process.exit(0);}
const stamp=new Date().toISOString().replace(/[:.]/g,'-'),backup=join(homedir(),'Library/Application Support/MusicOS/Development/checkpoints',`vesktop-theme-${stamp}`);await mkdir(backup,{recursive:true,mode:0o700});
await copyFile(destination,join(backup,'quickCss.css'));await chmod(join(backup,'quickCss.css'),0o600);
const css=await readFile(join(root,'extras/vesktop/MusicOS.css'),'utf8'),font=(await readFile(join(root,'assets/fonts/Figtree-Variable.ttf'))).toString('base64');
const license=await readFile(join(root,'assets/fonts/Figtree-OFL.txt'),'utf8');
const output=`/* ${license} */\n@font-face { font-family: "MusicOS Sans"; src: url("data:font/ttf;base64,${font}") format("truetype"); font-weight: 300 900; font-style: normal; font-display: swap; }\n${css}`;
await writeFile(destination+'.pending',output,{mode:0o600});await rename(destination+'.pending',destination);await copyFile(join(root,'assets/fonts/Figtree-OFL.txt'),join(backup,'Figtree-OFL.txt'));console.log(JSON.stringify({installed:destination,previous_css:join(backup,'quickCss.css'),network_fonts:false}));
