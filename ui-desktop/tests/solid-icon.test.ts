import {test,expect} from 'vitest';
import {solidIcon} from '../src/AssetIcon';
test('only playback and collection controls retain themed shading',()=>{
 const source='<svg><defs><linearGradient id="shade"><stop stop-color="#fff"/></linearGradient><clipPath id="clip"><path d="M0 0"/></clipPath></defs><use fill="url(#shade)" clip-path="url(#clip)" href="#shape"/><path id="shape" d="M1 2L3 4"/></svg>';
 const flat=solidIcon(source,'CollectionShuffle');expect(flat).toContain('linearGradient');expect(flat).toContain('var(--shuffle-top,var(--control-surface))');expect(flat).toContain('clip-path="url(#clip)"');expect(flat).toContain('d="M1 2L3 4"');expect(solidIcon(source,'Storage')).toContain('fill="currentColor"');expect(solidIcon(source,'Storage')).not.toContain('linearGradient');
});

test('refreshed Sketch masters stay visible and transparent without changing path geometry',async()=>{
 const sources=import.meta.glob('../../assets/icons/ui/*.svg',{query:'?raw',import:'default',eager:true}) as Record<string,string>;
 for(const name of ['Library','sync','tinySync','Album','SmartPlaylist','Metadata','Settings','Visualizer','Open In Finder','Open In Folder','Lyrics','Cloud','Drive','Add_circuled']){
  const svg=sources[`../../assets/icons/ui/${name}.svg`];expect(svg, name).toBeTruthy();
  const parser=new DOMParser(),before=parser.parseFromString(svg,'image/svg+xml'),after=parser.parseFromString(solidIcon(svg,name),'image/svg+xml');
  expect(before.querySelector('parsererror'),name).toBeNull();expect(before.querySelector('svg')?.getAttribute('viewBox')).toMatch(/^0 0 (32 32|8 8)$/);
  expect(before.querySelector('use[fill="#FFFFFF"]'),name).toBeNull();expect(before.querySelectorAll('g path,g rect,g polyline,g circle,g line,g ellipse,g polygon').length,name).toBeGreaterThan(0);
  expect([...after.querySelectorAll('path')].map(n=>n.getAttribute('d')),name).toEqual([...before.querySelectorAll('path')].map(n=>n.getAttribute('d')));
 }
 const sync=sources['../../assets/icons/ui/sync.svg'];expect(sync).toContain('C1.57857264,-6.57857264');expect(sync).not.toContain('stroke-width="4"');
});

test('Library preserves the three overlapping Sketch faces and two-unit strokes',()=>{
 const sources=import.meta.glob('../../assets/icons/ui/Library.svg',{query:'?raw',import:'default',eager:true}) as Record<string,string>;
 const svg=new DOMParser().parseFromString(solidIcon(sources['../../assets/icons/ui/Library.svg'],'Library'),'image/svg+xml');
 expect(svg.querySelectorAll('g > path')).toHaveLength(3);
 for(const path of svg.querySelectorAll('g > path')){expect(path.getAttribute('stroke-width')).toBe('2');expect(path.getAttribute('fill')).toBe('var(--icon-surface,var(--control-surface))');}
});
