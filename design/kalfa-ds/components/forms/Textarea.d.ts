export interface TextareaProps {
  value?: string;
  defaultValue?: string;
  placeholder?: string;
  rows?: number;
  /** Keep the copper border on permanently — used by the composer. */
  accent?: boolean;
  disabled?: boolean;
  onChange?: (e: React.ChangeEvent<HTMLTextAreaElement>) => void;
  /** Control tray rendered inside the field frame, below the text. */
  footer?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function Textarea(props: TextareaProps): JSX.Element;
