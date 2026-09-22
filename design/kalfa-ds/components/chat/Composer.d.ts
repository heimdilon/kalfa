/**
 * The always-copper input at the foot of a session: prompt + model + permission
 * mode in one frame, so the user can never send without seeing both settings.
 * @startingPoint section="Chat" subtitle="Prompt field with model and permission mode" viewport="700x150"
 */
export interface ComposerProps {
  placeholder?: string;
  value?: string;
  onChange?: (e: React.ChangeEvent<HTMLTextAreaElement>) => void;
  model?: string;
  models?: string[];
  onModelChange?: (model: string) => void;
  /** Permission mode: İnceleme | Sıkı | Proje | Serbest. */
  mode?: string;
  modes?: string[];
  onModeChange?: (mode: string) => void;
  onSend?: () => void;
  disabled?: boolean;
  style?: React.CSSProperties;
}
export declare function Composer(props: ComposerProps): JSX.Element;
