import type {CSSProperties} from 'react';
import {readableInk} from './appearance';
import type {Palette} from './types';
/** A player owns its palette; browsing a different release cannot recolor it. */
export function playerTheme(palette:Palette|null):CSSProperties{
 const accent=palette?.accent??'#0064df',ink=palette?.text??'#12202a';
 return {
  '--surface-color':palette?.background??'#f7fbff',
  '--blob-color':palette?.blob??'#e6f1fc',
  '--accent':accent,'--accent-ink':readableInk(accent),'--ink':ink,
  '--playback-tone':accent,'--icon-fill':ink,'--icon-stroke':ink,
  '--control-surface':'color-mix(in srgb,var(--surface-color) 78%,var(--blob-color))',
  '--control-border':'color-mix(in srgb,var(--ink) 18%,transparent)',
  '--secondary-ink':'color-mix(in srgb,var(--ink) 82%,var(--surface-color))',
  color:ink,background:palette?.background??'#f7fbff',
 } as CSSProperties;
}
