import {defineConfig} from '@playwright/test';
import {fileURLToPath} from 'node:url';
export default defineConfig({webServer:{command:'node node_modules/vite/bin/vite.js --config ui-desktop/vite.visualizer-proof.config.ts',cwd:fileURLToPath(new URL('..',import.meta.url)),url:'http://127.0.0.1:1422',reuseExistingServer:true},testDir:'./tests',testMatch:'visualizer*.spec.ts',workers:1,use:{baseURL:'http://127.0.0.1:1422',viewport:{width:1280,height:800},screenshot:'only-on-failure'},outputDir:'../out/visualizer-layout-check',reporter:'list'});
