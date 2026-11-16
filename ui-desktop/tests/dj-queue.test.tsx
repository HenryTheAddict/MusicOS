import {render, screen, within} from '@testing-library/react';
import {expect, test} from 'vitest';
import {QueueRows} from '../src/components';
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
