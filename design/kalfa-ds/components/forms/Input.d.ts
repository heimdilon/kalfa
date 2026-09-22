export interface InputProps {
  value?: string;
  defaultValue?: string;
  placeholder?: string;
  /** Lucide name, e.g. "search" for the session filter. */
  iconLeft?: string;
  iconRight?: string;
  type?: 'text' | 'password' | 'search';
  /** Monospace value — API keys, request ids, paths. */
  mono?: boolean;
  size?: 'md' | 'lg';
  invalid?: boolean;
  disabled?: boolean;
  onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
  style?: React.CSSProperties;
}
export declare function Input(props: InputProps): JSX.Element;
