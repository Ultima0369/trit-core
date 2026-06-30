// Mock for react-globe.gl in jsdom tests.
// Renders a stable element with data-testid so Earth.test.tsx can assert engine state.
import { forwardRef } from 'react';

const Globe = forwardRef<any, any>((_props, _ref) => (
  <div data-testid="globe-gl" />
));
Globe.displayName = 'Globe';

export default Globe;
export const GlobeMethods = {};
