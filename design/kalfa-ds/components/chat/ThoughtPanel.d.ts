export interface ThoughtPanelProps {
  /** Step count used in the default Turkish summary line. */
  steps?: number;
  /** Override the summary line entirely. */
  summary?: string;
  /** Kalfa ships this closed — reasoning is offered, not pushed. */
  defaultOpen?: boolean;
  children?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function ThoughtPanel(props: ThoughtPanelProps): JSX.Element;
