export interface FileTreeItem {
  name: string;
  /** Unique path; compared against activePath. */
  path?: string;
  type?: 'file' | 'dir';
  /** Indent level; the tree is pre-flattened, not nested. */
  depth?: number;
  /** Directories only — controls the chevron and folder glyph. */
  open?: boolean;
}
export interface FileTreeProps {
  items: FileTreeItem[];
  activePath?: string;
  onSelect?: (item: FileTreeItem) => void;
  style?: React.CSSProperties;
}
export declare function FileTree(props: FileTreeProps): JSX.Element;
