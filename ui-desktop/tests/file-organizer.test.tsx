import { StrictMode } from 'react';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import FileOrganizer, { type OrganizationPreview } from '../src/FileOrganizer';
import type { Task } from '../src/types';

const bridge = vi.hoisted(() => ({ call: vi.fn(), subscribe: vi.fn() }));
vi.mock('../src/bridge', () => ({ ...bridge, errorText: String }));
vi.mock('../src/AssetIcon', () => ({ AssetIcon: () => null }));

const preview = {
  token: 'reviewed',
  moves: [
    { id: 1, from: '/Music/Downloads/01.flac', to: '/Music/Artist/Album/01.flac' },
    { id: 2, from: 'C:\\Music\\Downloads\\02.flac', to: 'C:\\Music\\Artist\\Album\\02 (2).flac' },
  ],
  skipped: 1,
  undo_available: 3,
} satisfies OrganizationPreview;

let emit: (progress: Partial<Task>) => void;

function delayed<T>() {
  let resolve!: (result: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}

beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () { this.open = true; };
  HTMLDialogElement.prototype.close = function () { this.open = false; };
  bridge.subscribe.mockImplementation(async (_event, listener) => {
    emit = task => listener({ kind: 'organize', running: true, processed: 1, total: 2, ...task });
    return () => {};
  });
});

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

test('previews once in Strict Mode, isolates progress by phase and displays collision renames', async () => {
  const planned = delayed<OrganizationPreview>();
  const moved = delayed<{ count: number }>();
  const onRefresh = vi.fn().mockResolvedValue(undefined);
  bridge.call.mockReturnValueOnce(planned.promise).mockReturnValueOnce(moved.promise)
    .mockResolvedValueOnce({ ...preview, moves: [], undo_available: 5 });

  render(<StrictMode><FileOrganizer onClose={vi.fn()} onRefresh={onRefresh} /></StrictMode>);
  const checking = await screen.findByRole('progressbar', { name: 'Checking files for your preview' });
  await waitFor(() => expect(bridge.call).toHaveBeenCalledExactlyOnceWith('organize_files', { action: 'preview' }));
  expect(checking).not.toHaveAttribute('aria-valuenow');
  act(() => emit({ kind: 'rescan', processed: 4, total: 10 }));
  expect(checking).not.toHaveAttribute('aria-valuenow');
  act(() => emit({ processed: 5, total: 8 }));
  expect(checking).toHaveAttribute('aria-valuenow', '5');
  act(() => emit({ running: false, message: 'File organization finished' }));
  expect(screen.queryByText('File organization finished')).not.toBeInTheDocument();

  await act(async () => planned.resolve(preview));
  expect(screen.getByText('/Music/Downloads')).toBeInTheDocument();
  expect(screen.getByText('/Music/Artist/Album')).toBeInTheDocument();
  expect(screen.getByText('C:\\Music\\Artist\\Album\\02 (2).flac')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Move 2 files' }));
  expect(bridge.call).toHaveBeenLastCalledWith('organize_files', { action: 'apply', token: 'reviewed' });
  const moving = screen.getByRole('progressbar', { name: 'Moving files' });
  expect(moving).not.toHaveAttribute('aria-valuenow');
  expect(screen.queryByRole('list', { name: 'Proposed moves' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Undo organization (3)' })).not.toBeInTheDocument();
  act(() => emit({}));
  expect(moving).toHaveAttribute('aria-valuemax', '2');
  expect(moving).toHaveAttribute('aria-valuenow', '1');

  await act(async () => moved.resolve({ count: 2 }));
  expect(onRefresh).toHaveBeenCalledOnce();
  expect(screen.getByText('2 files organized.')).toBeInTheDocument();
  expect(screen.getByRole('heading', { name: 'No files to move' })).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Undo organization (5)' })).toBeEnabled();
  expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: /stop/i })).not.toBeInTheDocument();
});

test('a cancelled check requires a new preview before any move', async () => {
  const planned = delayed<OrganizationPreview>();
  bridge.call.mockReturnValueOnce(planned.promise).mockResolvedValueOnce(undefined).mockResolvedValueOnce(preview);
  render(<FileOrganizer onClose={vi.fn()} onRefresh={vi.fn()} />);
  await waitFor(() => expect(bridge.call).toHaveBeenCalledOnce());

  fireEvent.click(screen.getByRole('button', { name: 'Stop preview' }));
  expect(bridge.call).toHaveBeenLastCalledWith('cancel_task');
  expect(screen.getByRole('button', { name: 'Stop preview' })).toBeDisabled();
  expect(screen.getByRole('status')).toHaveTextContent('Stopping preview');

  await act(async () => planned.resolve({ ...preview, moves: preview.moves.slice(0, 1) }));
  expect(screen.getByRole('status')).toHaveTextContent('Preview stopped');
  expect(screen.queryByRole('button', { name: /^Move/ })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Preview again' }));
  expect(await screen.findByRole('button', { name: 'Move 2 files' })).toBeEnabled();
});

test('stopping a move reports completed work and offers the refreshed undo count', async () => {
  const moved = delayed<{ count: number }>();
  bridge.call.mockResolvedValueOnce(preview).mockReturnValueOnce(moved.promise)
    .mockResolvedValueOnce(undefined)
    .mockResolvedValueOnce({ ...preview, moves: preview.moves.slice(1), undo_available: 4 });
  render(<FileOrganizer onClose={vi.fn()} onRefresh={vi.fn().mockResolvedValue(undefined)} />);
  fireEvent.click(await screen.findByRole('button', { name: 'Move 2 files' }));
  fireEvent.click(screen.getByRole('button', { name: 'Stop after this file' }));
  await act(async () => moved.resolve({ count: 1 }));
  expect(screen.getByText('Stopped. 1 file organized.')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Undo organization (4)' })).toBeEnabled();
  expect(screen.getByRole('button', { name: 'Move 1 file' })).toBeEnabled();
});

test('pending recovery stays reachable and blocks new moves', async () => {
  bridge.call.mockResolvedValueOnce({ ...preview, undo_available: 0, has_pending_recovery: true })
    .mockResolvedValueOnce({ count: 0 }).mockResolvedValueOnce(preview);
  render(<FileOrganizer onClose={vi.fn()} onRefresh={vi.fn().mockResolvedValue(undefined)} />);
  const retry = await screen.findByRole('button', { name: 'Retry interrupted file work' });
  expect(screen.queryByRole('button', { name: /^(Move|Undo)/ })).not.toBeInTheDocument();
  fireEvent.click(retry);
  expect(bridge.call).toHaveBeenLastCalledWith('organize_files', { action: 'recover', token: 'reviewed' });
  expect(await screen.findByRole('button', { name: 'Move 2 files' })).toBeEnabled();
  expect(screen.getByText('Interrupted file work checked.')).toBeInTheDocument();
});

test('a move with pending cleanup can be undone instead of retried', async () => {
  bridge.call.mockResolvedValueOnce({ ...preview, undo_available: 1, has_pending_recovery: true })
    .mockResolvedValueOnce({ count: 1 }).mockResolvedValueOnce({ ...preview, undo_available: 0 });
  render(<FileOrganizer onClose={vi.fn()} onRefresh={vi.fn().mockResolvedValue(undefined)} />);
  const undo = await screen.findByRole('button', { name: 'Undo organization (1)' });
  expect(screen.queryByRole('button', { name: /^Move/ })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Retry interrupted file work' })).toBeEnabled();
  fireEvent.click(undo);
  expect(bridge.call).toHaveBeenLastCalledWith('organize_files', { action: 'undo', token: 'reviewed' });
  expect(await screen.findByRole('button', { name: 'Move 2 files' })).toBeEnabled();
  expect(screen.getByText('1 file restored.')).toBeInTheDocument();
});

test('a failed preview still allows recovery', async () => {
  bridge.call.mockRejectedValueOnce(new Error('Unable to read preview'));
  render(<FileOrganizer onClose={vi.fn()} onRefresh={vi.fn()} />);
  expect(await screen.findByRole('alert')).toHaveTextContent('Unable to read preview');
  expect(screen.getByRole('button', { name: 'Retry interrupted file work' })).toBeEnabled();
});
