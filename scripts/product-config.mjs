import fs from 'node:fs';
import assert from 'node:assert/strict';
const root=new URL('../',import.meta.url),read=path=>JSON.parse(fs.readFileSync(new URL(path,root),'utf8'));
const p=read('product.config.json');
assert.equal(p.schemaVersion,1);assert.match(p.name,/^[\p{L}\p{N} ._-]{1,40}$/u);assert.match(p.version,/^\d+\.\d+\.\d+$/);assert.equal(new URL(p.repositoryUrl).protocol,'https:');
assert.ok([80,100,125,150].includes(p.defaults.interfacePercent));assert.ok(['standard','compact','wide'].includes(p.defaults.miniPlayerDensity));
assert.ok(['system','light','dark'].includes(p.defaults.appearance.mode));assert.ok(['blue','artwork','custom'].includes(p.defaults.appearance.source));assert.match(p.defaults.appearance.accent,/^#[\da-f]{6}$/i);assert.ok(p.defaults.appearance.saturation>=0&&p.defaults.appearance.saturation<=150);assert.ok(p.defaults.appearance.tint>=0&&p.defaults.appearance.tint<=40);
for(const value of Object.values(p.storage))assert.ok(typeof value==='string'&&value.length>0&&!/[\\/]|^\.{1,2}$/.test(value),'Storage names must be single relative names');
for(const title of [p.defaults.libraryLabel,p.onboarding.desktopTitle,p.onboarding.phoneTitle])assert.ok(typeof title==='string'&&title.trim().length>0&&title.length<=80);
assert.ok(Number.isInteger(p.onboarding.version)&&p.onboarding.version>0);
const pkg=read('package.json'),tauri=read('src-tauri/tauri.conf.json');
if(process.argv.includes('--sync')){
 pkg.version=p.version;tauri.productName=p.name;tauri.version=p.version;
 for(const [path,value]of [['package.json',pkg],['src-tauri/tauri.conf.json',tauri]])fs.writeFileSync(new URL(path,root),JSON.stringify(value,null,2)+'\n');
 const html=new URL('ui-desktop/index.html',root);fs.writeFileSync(html,fs.readFileSync(html,'utf8').replace(/<title>[^<]*<\/title>/,`<title>${p.name}</title>`));
 const lock=new URL('package-lock.json',root);if(fs.existsSync(lock)){const value=JSON.parse(fs.readFileSync(lock,'utf8'));value.version=p.version;if(value.packages?.[''])value.packages[''].version=p.version;fs.writeFileSync(lock,JSON.stringify(value,null,2)+'\n');}
 const cargo=new URL('src-tauri/Cargo.toml',root);fs.writeFileSync(cargo,fs.readFileSync(cargo,'utf8').replace(/^(version = )"[^"]+"/m,`$1"${p.version}"`));
}
assert.equal(pkg.version,p.version,'Run npm run product:sync after changing the product version');assert.equal(tauri.version,p.version,'Run npm run product:sync');assert.equal(tauri.productName,p.name,'Run npm run product:sync');
console.log('Product configuration valid. Existing application IDs and profile locations retained.');
