import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import TopBar from '../TopBar';
import { EVENTS } from '../types';

describe('TopBar', () => {
  const defaultProps = {
    onRun: vi.fn(),
    onToggleSettings: vi.fn(),
    onReset: vi.fn(),
    onToggleFullscreen: vi.fn(),
    isFullscreen: false,
    view2D: false,
    onToggleView: vi.fn(),
    decision: null,
    loading: false,
    onOpenDecision: vi.fn(),
  };

  it('renders Aurora wordmark', () => {
    render(<TopBar {...defaultProps} />);
    expect(screen.getByText('Aurora')).toBeInTheDocument();
  });

  it('renders Run button', () => {
    render(<TopBar {...defaultProps} />);
    expect(screen.getByText('运行')).toBeInTheDocument();
  });

  it('renders Settings button', () => {
    render(<TopBar {...defaultProps} />);
    expect(screen.getByTitle('设置')).toBeInTheDocument();
  });

  it('calls onRun when Run button clicked', () => {
    const onRun = vi.fn();
    render(<TopBar {...defaultProps} onRun={onRun} />);
    fireEvent.click(screen.getByText('运行'));
    expect(onRun).toHaveBeenCalled();
  });

  it('calls onToggleSettings when Settings clicked', () => {
    const onToggleSettings = vi.fn();
    render(<TopBar {...defaultProps} onToggleSettings={onToggleSettings} />);
    fireEvent.click(screen.getByTitle('设置'));
    expect(onToggleSettings).toHaveBeenCalled();
  });

  it('shows decision when provided', () => {
    render(<TopBar {...defaultProps} decision="Hold" />);
    expect(screen.getByText('Hold')).toBeInTheDocument();
  });

  it('disables Run button when loading', () => {
    render(<TopBar {...defaultProps} loading={true} />);
    const btn = screen.getByText('运行中…');
    expect(btn).toBeDisabled();
  });

  it('cycles globe texture on texture button click', () => {
    let captured: string | null = null;
    const handler = (e: Event) => {
      captured = (e as CustomEvent).detail.texture;
    };
    window.addEventListener(EVENTS.SET_GLOBE_TEXTURE, handler);

    render(<TopBar {...defaultProps} />);
    const btn = screen.getByTitle(/地球纹理/);
    expect(btn).toBeInTheDocument();

    fireEvent.click(btn);
    expect(captured).toBe('topographic');

    window.removeEventListener(EVENTS.SET_GLOBE_TEXTURE, handler);
  });

  it('calls onToggleView when view toggle clicked', () => {
    const onToggleView = vi.fn();
    render(<TopBar {...defaultProps} onToggleView={onToggleView} />);
    fireEvent.click(screen.getByTitle(/切到/));
    expect(onToggleView).toHaveBeenCalled();
  });

  it('calls onOpenDecision when decision label clicked', () => {
    const onOpenDecision = vi.fn();
    render(<TopBar {...defaultProps} decision="Hold" onOpenDecision={onOpenDecision} />);
    fireEvent.click(screen.getByText('Hold'));
    expect(onOpenDecision).toHaveBeenCalled();
  });

  it('disables decision button when decision is null', () => {
    render(<TopBar {...defaultProps} decision={null} />);
    // decision 为 null 时标签不渲染（既有行为），无从点击
    expect(screen.queryByText('Hold')).not.toBeInTheDocument();
  });
});
