import type { Root } from 'mdast';
import { fromMarkdown } from 'mdast-util-from-markdown';
import { gfmFromMarkdown } from 'mdast-util-gfm';
import { gfm } from 'micromark-extension-gfm';

/**
 * Markdown metnini sözdizimi ağacına çevirir (CommonMark + GFM). Ağaç Svelte
 * bileşenleriyle çizilir; ham HTML düğümleri metin olarak kalır (1c tasarımı
 * §9.1). Kapanmamış kod bloğu CommonMark gereği metnin sonuna kadar sürer.
 */
export function cozumle(metin: string): Root {
  return fromMarkdown(metin, { extensions: [gfm()], mdastExtensions: [gfmFromMarkdown()] });
}
