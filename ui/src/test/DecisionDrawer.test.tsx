import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import DecisionDrawer from '../DecisionDrawer';
import type { PipelineResponse } from '../types';

const baseData: PipelineResponse = {
  detected_freq_hz: 2.0,
  decision: 'Hold',
  phase: 0.5,
  final_frame: 'Meta',
  signals: [
    { frame: 'Embodied', value: 'True', phase: 0.8 },
    { frame: 'Individual', value: 'False', phase: 0.2 },
  ],
  asi: 0.62,
  reminder_count: 0,
  active_shift_count: 0,
  conflicts: [],
  reminders: [],
  html: '',
  json: '',
};

describe('DecisionDrawer', () => {
  it('renders nothing when data is null', () => {
    const { container } = render(
      <DecisionDrawer open={true} onClose={vi.fn()} data={null} loading={false} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('renders nothing when closed', () => {
    const { container } = render(
      <DecisionDrawer open={false} onClose={vi.fn()} data={baseData} loading={false} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('renders decision head with decision, phase, frame', () => {
    render(<DecisionDrawer open={true} onClose={vi.fn()} data={baseData} loading={false} />);
    expect(screen.getByText('Hold')).toBeInTheDocument();
    expect(screen.getByText(/Phase 0\.50/)).toBeInTheDocument();
    expect(screen.getByText(/Meta/)).toBeInTheDocument();
  });

  it('calls onClose when close button clicked', () => {
    const onClose = vi.fn();
    render(<DecisionDrawer open={true} onClose={onClose} data={baseData} loading={false} />);
    fireEvent.click(screen.getByTitle('关闭'));
    expect(onClose).toHaveBeenCalled();
  });
});
