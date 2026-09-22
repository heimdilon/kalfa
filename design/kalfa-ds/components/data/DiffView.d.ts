export interface DiffLine {
  kind?: 'add' | 'del' | 'ctx';
  /** Line number in the file before the change. */
  old?: number | string;
  /** Line number after the change. */
  neu?: number | string;
  text: string;
}
export interface DiffViewProps {
  /** Unified hunk header, e.g. "@@ -42,10 +42,18 @@ async fn main()". */
  hunkHeader?: string;
  lines: DiffLine[];
  style?: React.CSSProperties;
}
export declare function DiffView(props: DiffViewProps): JSX.Element;
