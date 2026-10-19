import {test,expect} from 'vitest';
import {solidIcon} from '../src/AssetIcon';
test('solid icons preserve geometry and clipping references while removing shading',()=>{
 const source='<svg><defs><linearGradient id="shade"><stop stop-color="#fff"/></linearGradient><clipPath id="clip"><path d="M0 0"/></clipPath></defs><use fill="url(#shade)" clip-path="url(#clip)" href="#shape"/><path id="shape" d="M1 2L3 4"/></svg>';
 const flat=solidIcon(source,'CollectionShuffle');expect(flat).not.toContain('linearGradient');expect(flat).toContain('fill="var(--shuffle-face,var(--control-surface,#edf4f0))"');expect(flat).toContain('clip-path="url(#clip)"');expect(flat).toContain('d="M1 2L3 4"');expect(solidIcon(source,'Storage')).toContain('fill="currentColor"');
});
