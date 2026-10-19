export function egikKomutAyristir(metin: string): { komut: string; arguman: string } | null {
  if (!metin.startsWith('/')) {
    return null;
  }
  const kalan = metin.slice(1);
  const bosluk = kalan.search(/\s/);
  if (bosluk === -1) {
    return { komut: kalan, arguman: '' };
  }
  return {
    komut: kalan.slice(0, bosluk),
    arguman: kalan.slice(bosluk + 1).trim(),
  };
}
