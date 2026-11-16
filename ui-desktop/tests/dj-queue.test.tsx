import {fireEvent, render, screen, waitFor, within} from '@testing-library/react';
import {expect, test, vi} from 'vitest';
import {QueueRows} from '../src/components';
import AudioPanel from '../src/AudioPanel';
import {emptyPlayback} from '../src/bridge';
import type {QueueMix, Track} from '../src/types';

test('transition labels follow the exact neighboring queue entries after a move', () => {
 const track = (id: number, title: string) => ({id, title, artist: 'Artist', duration: 180}) as Track;
 const repeated = track(2, 'Repeated song');
 const other = track(3, 'Other song');
 const mixes: QueueMix[] = [
  {outgoing_entry: 10, incoming_entry: 20, kind: 'beat_match', seconds: 4.5},
  {outgoing_entry: 20, incoming_entry: 30, kind: 'outro_fade', seconds: 3},
  {outgoing_entry: 30, incoming_entry: 40, kind: 'crossfade', seconds: 2},
 ];
 const props = {mixes, precedingEntry: 10, scale: 1, current: null, onRemove: () => {}, onSelect: () => {}};
 const {rerender} = render(<QueueRows {...props} tracks={[repeated, other, repeated]} entryIds={[20, 30, 40]}/>);
 const plays = () => screen.getAllByRole('button', {name: /^Play .+ from queue$/});
 expect(plays()[0]).toHaveAccessibleDescription('Beat match, 4.5 seconds');
 expect(within(plays()[1]).getByText('Outro fade')).toBeVisible();
 expect(plays()[2]).toHaveAccessibleDescription('Crossfade, 2.0 seconds');

 rerender(<QueueRows {...props} tracks={[repeated, other, repeated]} entryIds={[40, 30, 20]}/>);
 expect(plays()[0]).not.toHaveAccessibleDescription();
 expect(plays()[1]).not.toHaveAccessibleDescription();
 expect(plays()[2]).not.toHaveAccessibleDescription();
});

test('the DJ switch sends one action and the orbit follows active playback', async () => {
 const player = {...emptyPlayback, dj_mode: true, is_playing: true, dj_transition: {
  outgoing_entry: 10, incoming_entry: 11, active: true, status: 'active' as const,
  outgoing_cue: 172, incoming_cue: 0, seconds: 5, tempo: 1,
 }};
 const onAction = vi.fn().mockResolvedValue(undefined);
 const props = {scale: 1, presentation: 'embedded' as const, onClose: () => {}, onSettings: () => {}, onAction};
 const {container, rerender} = render(<AudioPanel {...props} player={player}/>);
 const switchControl = screen.getByRole('switch', {name: 'DJ mode'});
 expect(switchControl).toBeChecked();
 expect(container.querySelector('.queue-dj')).toHaveAttribute('data-mixing', 'true');
 expect(container.querySelectorAll('.dj-orbit')).toHaveLength(2);
 expect([...container.querySelectorAll('.dj-letter > b')].map(letter=>letter.textContent).join('')).toBe('DJ mode');
 expect(screen.getByRole('status')).toHaveTextContent('Mixing');
 fireEvent.click(switchControl);
 expect(onAction).toHaveBeenCalledExactlyOnceWith({type: 'dj_mode', enabled: false});
 await waitFor(() => expect(switchControl).toBeEnabled());
 rerender(<AudioPanel {...props} player={{...player, is_playing: false}}/>);
 expect(container.querySelector('.queue-dj')).not.toHaveAttribute('data-mixing');
});
