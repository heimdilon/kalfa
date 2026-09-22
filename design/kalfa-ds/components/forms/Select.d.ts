export interface SelectOption { label: string; /** Right-aligned monospace hint — model task, shortcut, price. */ hint?: string }
export interface SelectProps {
  value: string;
  options?: Array<string | SelectOption>;
  /** Lucide name shown at the left, e.g. "box" for model, "shield" for permission mode. */
  icon?: string;
  tone?: 'default' | 'accent';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  onChange?: (value: string) => void;
  style?: React.CSSProperties;
}
export declare function Select(props: SelectProps): JSX.Element;
