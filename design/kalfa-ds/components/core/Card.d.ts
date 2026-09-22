export interface CardProps {
  title?: React.ReactNode;
  /** Monospace meta line under the title — path, count, duration. */
  subtitle?: React.ReactNode;
  /** Lucide name for the square glyph tile at the head. */
  icon?: string;
  /** Right-aligned header controls (IconButton, Badge). */
  actions?: React.ReactNode;
  tone?: 'default' | 'sunken' | 'accent' | 'danger';
  padded?: boolean;
  selected?: boolean;
  children?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function Card(props: CardProps): JSX.Element;
