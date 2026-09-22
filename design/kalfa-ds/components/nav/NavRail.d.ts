import * as React from 'react';
/** The fixed 213px left rail of the Kalfa desktop app. */
export interface NavRailProps {
  /** Brand block / wordmark area at the top. */
  header?: React.ReactNode;
  /** Pinned bottom area — usually settings and the model badge. */
  footer?: React.ReactNode;
  children?: React.ReactNode;
  /** Defaults to var(--rail-width) = 213px. */
  width?: string;
  style?: React.CSSProperties;
}
export declare function NavRail(props: NavRailProps): JSX.Element;
/** All-caps, letterspaced group label inside the rail. */
export interface NavSectionProps { label: string; children?: React.ReactNode; style?: React.CSSProperties }
export declare function NavSection(props: NavSectionProps): JSX.Element;