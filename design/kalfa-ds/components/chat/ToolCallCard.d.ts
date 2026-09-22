export interface ToolCallCardProps {
  /** What happened, past tense: "Dosya okundu", "Komut çalıştırıldı". */
  tool: string;
  /** Monospace target — path, command, glob. */
  target?: string;
  /** Monospace stats line: "412 satır · 18 KB". */
  meta?: React.ReactNode;
  status?: 'running' | 'done' | 'denied' | 'unknown';
  /** Lucide glyph for the tool family. */
  icon?: string;
  /** Output preview — usually a CodeBlock. */
  children?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function ToolCallCard(props: ToolCallCardProps): JSX.Element;
