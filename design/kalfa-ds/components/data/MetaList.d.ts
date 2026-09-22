export interface MetaItem {
  label: React.ReactNode;
  value: React.ReactNode;
  /** Render the value in mono — paths, sizes, ids, timestamps. */
  mono?: boolean;
  tone?: 'default' | 'accent';
}
export interface MetaListProps {
  items: MetaItem[];
  labelWidth?: number;
  dense?: boolean;
  style?: React.CSSProperties;
}
export declare function MetaList(props: MetaListProps): JSX.Element;
