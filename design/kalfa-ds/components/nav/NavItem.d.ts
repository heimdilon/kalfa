import * as React from 'react';
/**
 * A single row in the left rail or a nested tree list.
 * @startingPoint section="Navigation" subtitle="Rail row with active copper edge" viewport="260x120"
 */
export interface NavItemProps {
  /** Lucide glyph name shown at 18px. */
  icon?: string;
  /** Row label (Turkish, sentence case). */
  children?: React.ReactNode;
  /** Active rows get the copper tint plus a 3px copper left edge. */
  active?: boolean;
  /** Trailing mono count or hint. */
  badge?: React.ReactNode;
  /** Indent level; each level adds 14px of left padding. */
  depth?: number;
  onClick?: () => void;
  style?: React.CSSProperties;
}
export declare function NavItem(props: NavItemProps): JSX.Element;