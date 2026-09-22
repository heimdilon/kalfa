import * as React from 'react';
/** 44px title bar sitting above any panel or column. */
export interface PanelHeaderProps {
  /** Lucide glyph at 20px, muted. */
  icon?: string;
  title: React.ReactNode;
  /** Mono meta line beside the title (path, count, timestamp). */
  subtitle?: React.ReactNode;
  /** Right-aligned IconButtons / Selects. */
  actions?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function PanelHeader(props: PanelHeaderProps): JSX.Element;