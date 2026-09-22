export interface IconProps {
  /** Lucide icon name, kebab or Pascal ("file-text" | "FileText"). */
  name: string;
  /** Pixel box. 14 inline, 16 default UI, 18 nav, 20 headers. */
  size?: number;
  /** Lucide default is 2; Kalfa uses 1.75 for UI, 1.5 at 20px+. */
  strokeWidth?: number;
  color?: string;
  className?: string;
  style?: React.CSSProperties;
  /** Accessible label. Omit for decorative icons. */
  title?: string;
}
export declare function Icon(props: IconProps): JSX.Element;
