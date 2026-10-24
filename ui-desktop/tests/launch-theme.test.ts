import {afterEach,expect,test} from 'vitest';
import {launchAppearance,launchPreferences,prepareLaunchTheme,rememberLaunchAppearance} from '../src/launchTheme';
import {defaultPreferences} from '../src/bridge';
const appearance={mode:'dark',source:'blue',accent:'#0064df',saturation:100,tint:15} as const;
afterEach(()=>{delete window.__MUSICOS_LAUNCH_APPEARANCE__;localStorage.clear();document.documentElement.removeAttribute('style');});
test('native launch appearance wins over a stale browser cache and no name is invented',()=>{rememberLaunchAppearance({...appearance,mode:'light'});window.__MUSICOS_LAUNCH_APPEARANCE__=appearance;expect(launchAppearance()).toEqual(appearance);expect(launchPreferences(defaultPreferences).display_name).toBe('');prepareLaunchTheme();expect(document.documentElement.dataset.themeMode).toBe('dark');expect(document.documentElement.style.getPropertyValue('--shell-color')).not.toBe('#00baff');});
test('launch cache stores only appearance and rejects invalid colors or modes',()=>{rememberLaunchAppearance(appearance);expect(launchAppearance()).toEqual(appearance);expect(JSON.parse(localStorage.getItem('musicos-launch-appearance-v1')!)).toEqual(appearance);localStorage.setItem('musicos-launch-appearance-v1',JSON.stringify({...appearance,accent:'red'}));expect(launchAppearance().mode).toBe('system');});
