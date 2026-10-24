import {expect,it} from 'vitest';
import {miniDimensions} from '../src/MiniResize';
it('legacy slim requests use the artwork minimum and allow height resizing',()=>{expect(miniDimensions(200,700,true)).toEqual({width:244,height:700});expect(miniDimensions(450.4,20,true)).toEqual({width:450,height:320});});
it('expanded resizing preserves the minimum queue region',()=>{expect(miniDimensions(180,200,false)).toEqual({width:244,height:320});expect(miniDimensions(330,420,false)).toEqual({width:330,height:420});});

it('compact density has independent usable minimums without changing legacy requests',()=>{expect(miniDimensions(100,100,'compact')).toEqual({width:220,height:160});expect(miniDimensions(300,400,'compact')).toEqual({width:300,height:400});expect(miniDimensions(100,100,'standard')).toEqual({width:244,height:320});});
