export interface TurnSummaryProps {
  title?: string;
  /** Findings or changes, one per bullet. */
  items: React.ReactNode[];
  time?: string;
  /** Present only when the turn is actually reversible. */
  onUndo?: () => void;
  undoLabel?: string;
  /** Caveat line, e.g. "Kabuk etkileri geri alınamaz." */
  note?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function TurnSummary(props: TurnSummaryProps): JSX.Element;
