export interface MessageProps {
  author?: 'user' | 'kalfa';
  /** Defaults to "Sen" / "Kalfa". */
  name?: string;
  /** Short clock time, "14:28". */
  time?: string;
  /** Show the blinking copper caret while tokens are still arriving. */
  streaming?: boolean;
  children?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function Message(props: MessageProps): JSX.Element;
