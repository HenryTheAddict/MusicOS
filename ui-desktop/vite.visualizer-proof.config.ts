import {mergeConfig} from 'vite';
import {fileURLToPath} from 'node:url';
import base from './vite.config';
export default mergeConfig(base,{server:{port:1422},plugins:[{name:'isolated-visualizer-fixtures',enforce:'pre',resolveId(source:string){if(source==='./bridge')return fileURLToPath(new URL('./tests/fixtures/visualizer-bridge.ts',import.meta.url));}}]});
