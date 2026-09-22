export interface CodeBlockProps {
  code: string;
  /** Number shown on the first row — tool output is usually a slice of a file. */
  startLine?: number;
  showLineNumbers?: boolean;
  /** Renders the "···" continuation marker used when output was clipped. */
  truncated?: boolean;
  maxHeight?: number | string;
  style?: React.CSSProperties;
}
export declare function CodeBlock(props: CodeBlockProps): JSX.Element;
