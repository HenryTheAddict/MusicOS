import {defineConfig} from '@playwright/test';
import {fileURLToPath} from 'node:url';
export default defineConfig({testDir:'./tests',testMatch:'**/*.spec.ts',workers:1,timeout:30000,use:{browserName:'webkit',hasTouch:true,isMobile:true,baseURL:'http://127.0.0.1:1433',viewport:{width:430,height:932},screenshot:'only-on-failure',trace:'retain-on-failure'},webServer:{command:'node node_modules/vite/bin/vite.js --config ios/vite.config.ts',cwd:fileURLToPath(new URL('..',import.meta.url)),url:'http://127.0.0.1:1433',reuseExistingServer:true},outputDir:'../out/phone-touch-qa',reporter:'list'});
