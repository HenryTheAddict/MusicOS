import {chromium} from '@playwright/test';
import {readdir, writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';

const root=fileURLToPath(new URL('..',import.meta.url));
const covers=['bubblewrap-lagoon','cardboard-carnivals','mechanical-marshmallows','clockwork-confetti','teapot-tornadoes','sleepy-kazoo','dreaming-teacups','marshmallow-observatory','teacup-parade','Overcast','umbrella-acrobatics','marmalade-meteorites','sky','tritower','sparklist_1'];
const palettes=[['#182642','#ef8979','#f6d9aa'],['#352752','#b797ef','#f3b6bf'],['#103f47','#61c9b6','#f6c764'],['#3c2240','#f192ce','#e3d4ff'],['#243c27','#9ed578','#e8e5a2'],['#542824','#ed9c5f','#ffe0ab'],['#182d61','#7199ef','#b7e3ed'],['#42335d','#d1a3e1','#f4b29a'],['#174e50','#84daca','#f9d898'],['#513149','#f288ad','#d4c7ef'],['#293c58','#a1c8e9','#e3d4a0'],['#614027','#f6b960','#f7e5b3']];
const browser=await chromium.launch({headless:true});
try {
  const page=await browser.newPage();
  const targets=covers.map(name=>`assets/covers/${name}.png`);
  targets.push('assets/artists/moonspindle.png');
  for(const file of (await readdir(path.join(root,'assets/covers/playlists'))).sort()) {
    if(file.endsWith('.png'))targets.push(`assets/covers/playlists/${file}`);
  }
  for(let index=0;index<targets.length;index++) {
    const file=targets[index];
    const data=await page.evaluate(({index,palette})=>{
      const canvas=document.createElement('canvas');canvas.width=canvas.height=640;
      const ctx=canvas.getContext('2d');
      const [dark,mid,light]=palette;
      const gradient=ctx.createLinearGradient(0,0,640,640);gradient.addColorStop(0,dark);gradient.addColorStop(1,mid);
      ctx.fillStyle=gradient;ctx.fillRect(0,0,640,640);
      ctx.save();ctx.translate(320,320);ctx.rotate((index%7-3)*Math.PI/12);
      ctx.fillStyle=light;ctx.globalAlpha=.9;
      if(index%3===0){ctx.beginPath();ctx.arc(0,-65,170,0,Math.PI*2);ctx.fill();ctx.fillStyle=dark;ctx.beginPath();ctx.arc(65,-100,145,0,Math.PI*2);ctx.fill();}
      else if(index%3===1){for(let n=0;n<4;n++){ctx.globalAlpha=.9-n*.15;ctx.fillRect(-200+n*60,-200+n*65,260,260);}}
      else {ctx.lineWidth=35;ctx.strokeStyle=light;for(let n=0;n<4;n++){ctx.globalAlpha=.85-n*.15;ctx.beginPath();ctx.ellipse(0,0,75+n*48,210,0,0,Math.PI*2);ctx.stroke();}}
      ctx.restore();ctx.globalAlpha=.45;ctx.strokeStyle=light;ctx.lineWidth=2;
      for(let y=475;y<590;y+=19){ctx.beginPath();ctx.moveTo(45,y);ctx.lineTo(595,y-90);ctx.stroke();}
      return {png:canvas.toDataURL('image/png').split(',')[1],webp:canvas.toDataURL('image/webp',.9).split(',')[1]};
    },{index,palette:palettes[index%palettes.length]});
    await writeFile(path.join(root,file),Buffer.from(data.png,'base64'));
    if(file.includes('/playlists/'))await writeFile(path.join(root,path.dirname(file),'thumbnails',path.basename(file,'.png')+'.webp'),Buffer.from(data.webp,'base64'));
  }
  console.log(`Generated ${targets.length} original abstract demo covers (plus playlist previews).`);
} finally {await browser.close();}
