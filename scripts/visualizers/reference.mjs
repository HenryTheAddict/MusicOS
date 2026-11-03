import {createRequire} from 'node:module';
import {fileURLToPath} from 'node:url';
const {chromium}=createRequire(import.meta.url)('@playwright/test');
import fs from 'node:fs/promises';
const root=fileURLToPath(new URL('../../',import.meta.url)).replace(/\/$/,'');
const frames=Number(process.env.FRAMES??180),output=process.env.REFERENCE_OUTPUT??`${root}/out/visualizer-reference/butterchurn-renders`;
const browser=await chromium.launch({headless:true,args:['--use-angle=metal']});
try{await fs.mkdir(output,{recursive:true});
 const manifest=JSON.parse(await fs.readFile(`${root}/assets/visualizers/manifest.json`,'utf8'));
 for(const preset of manifest.presets){const page=await browser.newPage({viewport:{width:640,height:480}});page.on('pageerror',e=>console.error(e.message));page.on('console',m=>{if(m.type()==='error')console.error(m.text());});await page.goto('http://127.0.0.1:17022/reference.html');await page.waitForFunction(()=>!!window.renderPreset);
 const source=JSON.parse(await fs.readFile(`${root}/assets/visualizers/${preset.source}`,'utf8'));
 const result=await page.evaluate(async({source,frames})=>await window.renderPreset(source,frames),{source,frames});
 await fs.writeFile(`${output}/${preset.id}.png`,Buffer.from(result.image.split(',')[1],'base64'));delete result.image;
 await fs.writeFile(`${output}/${preset.id}.json`,JSON.stringify(result));console.log(preset.id,Math.round(result.times.reduce((a,b)=>a+b,0)/result.times.length*100)/100,'ms scripting');await page.close();}
}finally{await browser.close();}
