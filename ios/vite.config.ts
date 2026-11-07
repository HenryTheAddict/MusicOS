import {defineConfig} from 'vite';
import react from '@vitejs/plugin-react';
import {fileURLToPath} from 'node:url';
export default defineConfig({
 root:fileURLToPath(new URL('./ui',import.meta.url)),plugins:[react(),{
  name:'companion-bridge',enforce:'pre',resolveId(source,importer){
   if(source==='./bridge'&&importer?.includes('/ui-desktop/src/'))return fileURLToPath(new URL('./ui/bridge.ts',import.meta.url));
  }
 }],server:{host:'127.0.0.1',port:1433,strictPort:true,fs:{allow:['../..']}},
 build:{outDir:'../dist',emptyOutDir:true}
});
