/**
 * Provenance chip. Kalfa's core promise is that the user can see what left the
 * machine — every content item in a transmission row is one of these.
 */
export interface SourceTagProps {
  kind?: 'dosya' | 'kabuk' | 'web' | 'eklenti' | 'rag' | 'mesaj' | 'gecmis';
  /** The payload text: "src/main.rs:1–120", "cargo test", "+34 önceki mesaj". */
  value: string;
  /** Green dot instead of the glyph — content captured from a running process. */
  live?: boolean;
  muted?: boolean;
  onClick?: () => void;
  style?: React.CSSProperties;
}
export declare function SourceTag(props: SourceTagProps): JSX.Element;
