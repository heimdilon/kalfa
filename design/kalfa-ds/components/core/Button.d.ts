/**
 * Kalfa's action button. Consent-bearing actions use the approve/caution/danger
 * trio so a decision never reads as a neutral UI control.
 * @startingPoint section="Core" subtitle="Action buttons in every Kalfa variant" viewport="700x180"
 */
export interface ButtonProps {
  /** primary = the single committing action · secondary = default · ghost = toolbar/icon-adjacent · approve/caution/danger = permission decisions only. */
  variant?: 'primary' | 'secondary' | 'ghost' | 'approve' | 'caution' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  /** Lucide icon name rendered before the label. */
  iconLeft?: string;
  iconRight?: string;
  fullWidth?: boolean;
  disabled?: boolean;
  loading?: boolean;
  type?: 'button' | 'submit';
  onClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  children?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function Button(props: ButtonProps): JSX.Element;
