import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({
    detected_freq_hz: 2.0,
    decision: 'Hold',
    phase: 0.5,
    final_frame: 'Meta',
    signals: [
      { frame: 'Science', value: 'True', phase: 0.8 },
      { frame: 'Individual', value: 'False', phase: 0.35 },
    ],
    asi: 0.5,
    reminder_count: 0,
    active_shift_count: 0,
    conflicts: [],
    reminders: [],
    html: '<p>test</p>',
    json: '{}',
  }),
}));

vi.mock('../Earth', () => ({ default: () => <div /> }));

import App, { readStored } from '../App';

describe('readStored', () => {
  it('returns fallback when key is absent (getItem returns null)', () => {
    // Regression: Number(null)=0 passed the >=0 guard, returning 0 instead of
    // fallback — this zeroed --font-scale and made all button text 0px/invisible.
    localStorage.clear();
    expect(readStored('aurora.fontScale', 1)).toBe(1);
    expect(readStored('aurora.rotationSpeed', 6)).toBe(6);
  });

  it('returns parsed value when key is present', () => {
    localStorage.clear();
    localStorage.setItem('aurora.fontScale', '1.25');
    expect(readStored('aurora.fontScale', 1)).toBe(1.25);
  });

  it('allows stored 0 (valid for rotationSpeed)', () => {
    localStorage.clear();
    localStorage.setItem('aurora.rotationSpeed', '0');
    expect(readStored('aurora.rotationSpeed', 6)).toBe(0);
  });
});

describe('App', () => {
  it('renders the Aurora branding', () => {
    render(<App />);
    const elements = screen.getAllByText('Aurora');
    expect(elements.length).toBeGreaterThanOrEqual(1);
  });

  it('renders the Run button', () => {
    render(<App />);
    // App auto-runs on mount, so the button may show "运行中…"; match either.
    expect(screen.getByTitle('运行分析')).toBeInTheDocument();
  });

  it('renders the Esc hint', () => {
    render(<App />);
    expect(screen.getByText(/Esc 退出/)).toBeInTheDocument();
  });

  it('renders the decision indicator after run resolves', async () => {
    const { container } = render(<App />);
    // App auto-runs on mount; once the pipeline resolves, the TopBar shows the decision.
    await screen.findByText('Hold', undefined, { timeout: 2000 });
    expect(container.textContent).toMatch(/Hold/);
  });
});
