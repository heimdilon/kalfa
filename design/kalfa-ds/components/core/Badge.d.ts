export interface BadgeProps {
  tone?: 'neutral' | 'accent' | 'ok' | 'warn' | 'danger' | 'info';
  /** Lucide icon name shown before the label. */
  icon?: string;
  /** Show a filled status dot instead of an icon. */
  dot?: boolean;
  shape?: 'pill' | 'square';
  children?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function Badge(props: BadgeProps): JSX.Element;
