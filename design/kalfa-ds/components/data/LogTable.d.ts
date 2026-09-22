export interface LogColumn {
  key: string;
  label: string;
  /** CSS grid track, e.g. "120px" or "1.4fr". */
  width?: string;
  mono?: boolean;
  muted?: boolean;
}
export interface LogRowData {
  id: string;
  /** Reserved key: rendered as a glyph tile + title + note. */
  purpose?: string;
  purposeNote?: string;
  purposeIcon?: string;
  purposeTone?: 'neutral' | 'accent' | 'ok' | 'warn' | 'danger' | 'info';
  [key: string]: any;
}
export interface LogTableProps {
  columns: LogColumn[];
  rows: LogRowData[];
  activeId?: string;
  onSelect?: (row: LogRowData) => void;
  style?: React.CSSProperties;
}
export declare function LogTable(props: LogTableProps): JSX.Element;
