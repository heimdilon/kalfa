import * as React from 'react';
/** The 46px bottom bar: isolation state, model, token spend, connection. */
export interface StatusBarProps {
  left?: React.ReactNode;
  right?: React.ReactNode;
  children?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function StatusBar(props: StatusBarProps): JSX.Element;
/** One mono fact inside the status bar. */
export interface StatusItemProps {
  icon?: string;
  tone?: 'ok' | 'warn' | 'danger' | 'info' | 'accent';
  children?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function StatusItem(props: StatusItemProps): JSX.Element;