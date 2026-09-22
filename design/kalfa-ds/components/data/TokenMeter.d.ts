export interface TokenSegment { label: string; value: number; /** Any color token; defaults to copper. */ color?: string }
export interface TokenMeterProps {
  segments: TokenSegment[];
  /** Context window size; the bar fills proportionally when given. */
  total?: number;
  /** Tokens used — defaults to the sum of segments. */
  used?: number;
  style?: React.CSSProperties;
}
export declare function TokenMeter(props: TokenMeterProps): JSX.Element;
