export interface SwitchProps {
  checked?: boolean;
  /** Label sits to the LEFT of the track in Kalfa (setting-first reading order). */
  label?: React.ReactNode;
  hint?: React.ReactNode;
  disabled?: boolean;
  onChange?: (next: boolean) => void;
  style?: React.CSSProperties;
}
export declare function Switch(props: SwitchProps): JSX.Element;
