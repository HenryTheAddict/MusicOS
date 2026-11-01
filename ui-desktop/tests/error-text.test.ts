import {expect,it} from 'vitest';
import {errorText} from '../src/bridge';
it('renders native Apple errors and nested failures without object coercion',()=>{
 expect(errorText({code:'operation_failed',message:'Apple Music has no exact recording match.'})).toBe('Apple Music has no exact recording match.');
 expect(errorText({error:{message:'Reconnect Apple Music.'}})).toBe('Reconnect Apple Music.');
 expect(errorText({})).toBe('Something went wrong. Please try again.');
 expect(errorText({message:'database is locked'})).toContain('finishing another change');
});
