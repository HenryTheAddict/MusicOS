import {expect,test} from 'vitest';
import {cssColorHex} from '../src/colorValue';
test('WebKit computed sRGB colors retain their actual theme instead of the pale fallback',()=>{expect(cssColorHex('color(srgb 0.6 0.4 0.5)','#f7fbff')).toBe('#996680');expect(cssColorHex('color(srgb 0.1 0.12 0.2 / 1)','#f7fbff')).toBe('#1a1f33');expect(cssColorHex('color(srgb 100% 0% 50%)','#f7fbff')).toBe('#ff0080');});
test('existing hex and RGB serializations retain the same channels',()=>{expect(cssColorHex(' #123abc ','#ffffff')).toBe('#123abc');expect(cssColorHex('rgb(153, 102, 128)','#ffffff')).toBe('#996680');expect(cssColorHex('rgb(60% 40% 50%)','#ffffff')).toBe('#996680');});
