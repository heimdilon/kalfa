export interface IconTileProps {
  icon: string;
  tone?: 'neutral' | 'accent' | 'ok' | 'warn' | 'danger' | 'info';
  /** Box size in px. 28 in dense rows, 32 default, 40 in headers. */
  size?: number;
  style?: React.CSSProperties;
}
export declare function IconTile(props: IconTileProps): JSX.Element;
