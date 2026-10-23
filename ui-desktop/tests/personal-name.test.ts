import {expect,test} from 'vitest';
import {personalName} from '../src/personalName';
test('personal labels use saved names and never guess during bootstrap',()=>{expect(personalName('Henry')).toBe("Henry's stuff");expect(personalName(' Zoë ')).toBe("Zoë's stuff");expect(personalName('Henry',false)).toBe('');expect(personalName('')).toBe('Your library');});
