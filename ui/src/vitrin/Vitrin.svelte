<!--
  Bileşen vitrini: tasarım sistemiyle görsel karşılaştırma için (1c tasarımı
  §11.5). Örnek veriler burada kalır; uygulamaya girmez.
-->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import CodeBlock from '../bilesenler/CodeBlock.svelte';
  import Composer from '../bilesenler/Composer.svelte';
  import DiffView from '../bilesenler/DiffView.svelte';
  import { diffCozumle } from '../bilesenler/diff';
  import IconButton from '../bilesenler/IconButton.svelte';
  import IconTile from '../bilesenler/IconTile.svelte';
  import Input from '../bilesenler/Input.svelte';
  import Message from '../bilesenler/Message.svelte';
  import MetaList from '../bilesenler/MetaList.svelte';
  import NavItem from '../bilesenler/NavItem.svelte';
  import NavRail from '../bilesenler/NavRail.svelte';
  import NavSection from '../bilesenler/NavSection.svelte';
  import PanelHeader from '../bilesenler/PanelHeader.svelte';
  import Select from '../bilesenler/Select.svelte';
  import SessionRow from '../bilesenler/SessionRow.svelte';
  import StatusBar from '../bilesenler/StatusBar.svelte';
  import StatusItem from '../bilesenler/StatusItem.svelte';
  import Textarea from '../bilesenler/Textarea.svelte';
  import ThoughtPanel from '../bilesenler/ThoughtPanel.svelte';
  import TokenMeter from '../bilesenler/TokenMeter.svelte';
  import ToolCallCard from '../bilesenler/ToolCallCard.svelte';
  import { KIP_ADI, MODEL_PROFILI_ADI, m } from '../dil/tr';
  import { KIPLER } from '../ipc/tipler';
  import Markdown from '../markdown/Markdown.svelte';

  const MARKDOWN = [
    '## Yapılanlar',
    '',
    '`src/main.rs` içinde **iki** değişiklik var:',
    '',
    '- [x] okuma',
    '- [ ] yazma',
    '',
    '| Dosya | Satır |',
    '|:--|--:|',
    '| main.rs | 42 |',
    '',
    '```rust',
    'fn main() {',
    '    println!("merhaba");',
    '}',
    '```',
    '',
    '[EVREN](https://evren.example) · [zararlı](javascript:alert(1)) · <b>ham html</b>',
    '',
    '![rozet](https://img.shields.io/badge/x-y-green)',
  ].join('\n');

  const DIFF = [
    '--- a/src/main.rs',
    '+++ b/src/main.rs',
    '@@ -1,3 +1,4 @@',
    ' fn main() {',
    '-    println!("merhaba");',
    '+    let ad = "Kalfa";',
    '+    println!("merhaba {ad}");',
    ' }',
  ].join('\n');

  const modeller = [
    { value: 'qwen3.8-flash-next', label: m(MODEL_PROFILI_ADI.Hizli), hint: 'qwen3.8-flash-next' },
    { value: 'deepseek-v4-flash', label: m(MODEL_PROFILI_ADI.Dengeli), hint: 'deepseek-v4-flash' },
    { value: 'glm-5.3', label: m(MODEL_PROFILI_ADI.Derin), hint: 'glm-5.3' },
    { value: 'auto', label: m(MODEL_PROFILI_ADI.Otomatik), hint: 'auto' },
  ];
  const kipler = KIPLER.map((k) => ({ value: k, label: m(KIP_ADI[k]) }));

  let model = $state('glm-5.3');
  let kip = $state('Siki');
  let metin = $state('');
  let calisiyor = $state(false);
  let secili = $state(true);
  let girdi = $state('');
  const baglanti = (adres: string) => console.info('baglanti_ac', adres);
</script>

<div class="vitrin">
  <NavRail label="Menü">
    {#snippet header()}<strong class="marka">Kalfa</strong>{/snippet}
    <NavItem icon="message-square" active badge={3}>Oturumlar</NavItem>
    <NavItem icon="key-round">Anahtarı değiştir</NavItem>
    <NavSection label="Çalışma alanları">
      <NavItem icon="folder" depth={1}>harness</NavItem>
    </NavSection>
  </NavRail>

  <div class="liste">
    <PanelHeader icon="list" title="Oturumlar" subtitle="4 kayıt" />
    <SessionRow title="Diff görünümü" preview="C:\projeler\harness" time="14:28" status="running" statusLabel="Çalışıyor" active />
    <SessionRow title="Onay bekleyen" time="14:10" status="waiting" statusLabel="Onay bekliyor" />
    <SessionRow title="Bitti" time="dün" status="done" statusLabel="Tamamlandı" />
    <SessionRow title="Yarım kalan" time="dün" status="partial" statusLabel="Yarım kaldı" />
    <SessionRow title="Hata" time="pzt" status="failed" statusLabel="Hata" />
  </div>

  <main class="icerik">
    <PanelHeader icon="message-square" title="Bileşen vitrini" subtitle="1c-3a">
      {#snippet actions()}
        <IconButton icon="plus" label="Yeni oturum" />
        <IconButton icon="x" label="Oturumu kapat" />
      {/snippet}
    </PanelHeader>

    <div class="kaydirma">
      <section>
        <h3>Düğmeler ve rozetler</h3>
        <div class="satir">
          <Button variant="primary" iconLeft="send">Gönder</Button>
          <Button>İkincil</Button>
          <Button variant="ghost">Hayalet</Button>
          <Button variant="approve" iconLeft="check">Tümünü onayla</Button>
          <Button variant="caution">Seçilenleri onayla</Button>
          <Button variant="danger" iconLeft="x">Reddet</Button>
          <Button loading>Bekleniyor</Button>
          <Button disabled>Kapalı</Button>
        </div>
        <div class="satir">
          <Badge>nötr</Badge>
          <Badge tone="accent" icon="box">glm-5.3</Badge>
          <Badge tone="ok" dot>Bağlı</Badge>
          <Badge tone="warn">Onay bekliyor</Badge>
          <Badge tone="danger" shape="square">Hata</Badge>
          <Badge tone="info">Bilgi</Badge>
          <IconTile icon="file-text" />
          <IconTile icon="hexagon" tone="accent" />
          <IconTile icon="triangle-alert" tone="warn" size={40} />
        </div>
      </section>

      <section>
        <h3>Formlar</h3>
        <div class="satir">
          <Checkbox label="src/main.rs" hint="+2 −1" checked={secili} onchange={(v) => (secili = v)} />
          <Checkbox label="Tümünü seç" indeterminate />
          <Input bind:value={girdi} placeholder="Oturum ara" iconLeft="search" />
          <Input type="password" mono placeholder="evren_llm_…" invalid />
          <Select label="Model" icon="box" tone="accent" value={model} options={modeller} onchange={(v) => (model = v)} />
        </div>
        <Textarea placeholder="Ret gerekçesi" rows={2} />
      </section>

      <section>
        <h3>Sohbet</h3>
        <Message author="user" time="14:28">Diff görünümünü ekler misin?</Message>
        <Message time="14:29" streaming={calisiyor}>
          <Markdown metin={MARKDOWN} akista={calisiyor} baglantiAc={baglanti} />
        </Message>
        <ThoughtPanel>
          <Markdown metin="Önce dosyayı okumalıyım." baglantiAc={baglanti} />
        </ThoughtPanel>
        <ToolCallCard tool="Dosya okundu" target="src/main.rs" meta="3 satır" statusLabel="Tamamlandı">
          <CodeBlock code={'fn main() {\n    println!("merhaba");\n}'} lang="rust" highlight startLine={1} />
        </ToolCallCard>
        <ToolCallCard tool="Dosya yazılıyor" target="src/lib.rs" status="running" statusLabel="Çalışıyor" icon="file-pen" />
        <ToolCallCard tool="Dosya yazılamadı" target="src/lib.rs" status="denied" statusLabel="Reddedildi" icon="file-pen" />
        <ToolCallCard tool="Sonucu bilinmiyor" status="unknown" statusLabel="Sonucu bilinmiyor" />
        <Composer
          bind:value={metin}
          models={modeller}
          {model}
          modes={kipler}
          mode={kip}
          running={calisiyor}
          onsend={() => (calisiyor = true)}
          oncancel={() => (calisiyor = false)}
          onmodelchange={(v) => (model = v)}
          onmodechange={(v) => (kip = v)}
        />
      </section>

      <section>
        <h3>Veri</h3>
        {#each diffCozumle(DIFF) as parca, i (i)}
          <DiffView hunkHeader={parca.baslik} lines={parca.satirlar} />
        {/each}
        <Card title="Onay isteği" subtitle="2 dosya" icon="file-pen" tone="accent">
          <MetaList
            items={[
              { label: 'Ne', value: 'src/main.rs dosyasını düzenler' },
              { label: 'Nerede', value: 'C:\\projeler\\harness\\src\\main.rs', mono: true },
              { label: 'Model', value: 'glm-5.3', tone: 'accent' },
            ]}
          />
        </Card>
        <TokenMeter
          segments={[
            { label: 'İstem', value: 12840 },
            { label: 'Tamamlama', value: 2210, color: 'var(--blue-200)' },
            { label: 'Düşünce', value: 930, color: 'var(--green-200)' },
          ]}
        />
      </section>

      <section class="kalfa-light belge-ornegi">
        <h3>Açık tema: şart metni</h3>
        <Markdown metin={MARKDOWN} belge baglantiAc={baglanti} />
      </section>
    </div>

    <StatusBar label="Durum çubuğu">
      {#snippet left()}
        <StatusItem icon="coins" title="Son gözlenen bakiye">1.250,00 CR · 14:29</StatusItem>
        <StatusItem icon="cpu">glm-5.3</StatusItem>
      {/snippet}
      {#snippet right()}
        <StatusItem icon="shield" tone="accent">Sıkı</StatusItem>
        <StatusItem tone="ok">15.980 token</StatusItem>
      {/snippet}
    </StatusBar>
  </main>
</div>

<style>
  .vitrin {
    display: flex;
    height: 100%;
  }
  .marka {
    font: var(--type-title);
    color: var(--text-title);
  }
  .liste {
    width: var(--list-width);
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 8px;
    background: var(--bg-panel);
    border-right: var(--border-width) solid var(--border-subtle);
  }
  .icerik {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .kaydirma {
    flex: 1;
    overflow: auto;
    padding: var(--pad-panel);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 32px;
  }
  h3 {
    margin: 0;
    font: var(--type-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .satir {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .belge-ornegi {
    padding: 20px;
    border-radius: var(--radius-xl);
    background: var(--bg-app);
    color: var(--text-body);
  }
</style>
