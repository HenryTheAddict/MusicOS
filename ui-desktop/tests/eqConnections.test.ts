import {expect,test} from 'vitest';
import {eqConnections} from '../src/eqConnections';
test('connections pass through each handle center with rounded tangents',()=>{expect(eqConnections([12,0,-12])).toBe('M 30 6 C 60 6, 60 34, 90 34 C 120 34, 120 62, 150 62');expect(eqConnections(Array(8).fill(0))).not.toMatch(/NaN|Infinity/);});
