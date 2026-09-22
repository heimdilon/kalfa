export interface CheckboxProps {
  checked?: boolean;
  /** Mixed state — the "Tümünü seç" header of a partially selected file list. */
  indeterminate?: boolean;
  label?: React.ReactNode;
  /** Monospace trailing note, e.g. "+12 −4". */
  hint?: React.ReactNode;
  disabled?: boolean;
  onChange?: (next: boolean) => void;
  style?: React.CSSProperties;
}
export declare function Checkbox(props: CheckboxProps): JSX.Element;
