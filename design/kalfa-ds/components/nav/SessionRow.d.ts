import * as React from 'react';
/**
 * A session entry in the 295px session list column.
 * @startingPoint section="Navigation" subtitle="Session list row with status dot" viewport="295x160"
 */
export interface SessionRowProps {
  title: React.ReactNode;
  /** Two-line clamped excerpt of the last turn. */
  preview?: React.ReactNode;
  /** Mono timestamp, right aligned. */
  time?: string;
  /** Status dot colour: running=amber, done=green, failed=red, idle=muted. */
  status?: 'running' | 'done' | 'failed' | 'idle';
  active?: boolean;
  onClick?: () => void;
  style?: React.CSSProperties;
}
export declare function SessionRow(props: SessionRowProps): JSX.Element;