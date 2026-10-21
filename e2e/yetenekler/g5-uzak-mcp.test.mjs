// Dış HTTP MCP servisi sabit; gerçek Tauri yönetici/keşif/mühür/sonuç akışı.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa } from '../yardimci/kalfa.mjs';

describe('G5 gerçek uzak MCP masaüstü', () => {
  let k, sunucu, cagrilar = [];
  before(async () => {
    sunucu = createServer(async (req, res) => {
      const parcalar = [];
      for await (const p of req) parcalar.push(p);
      const ham = Buffer.concat(parcalar).toString();
      if (!ham) { res.writeHead(204).end(); return; }
      const rpc = JSON.parse(ham);
      if (!('id' in rpc)) { res.writeHead(202).end(); return; }
      let result;
      if (rpc.method === 'initialize') result = { protocolVersion: '2025-03-26', capabilities: { tools: {} }, serverInfo: { name: 'g5-ui', version: '1.0' } };
      else if (rpc.method === 'tools/list') result = { tools: [{ name: 'kayit_getir', description: 'Kayıt getirin', inputSchema: { type: 'object', properties: { bilet: { type: 'integer' } }, required: ['bilet'] } }] };
      else if (rpc.method === 'tools/call') {
        cagrilar.push(rpc.params);
        result = { content: [{ type: 'text', text: `G5-UI-GERCEK-HTTP-${rpc.params.arguments.bilet}` }], isError: false };
      } else { res.writeHead(400).end(); return; }
      res.writeHead(200, { 'Content-Type': 'application/json', 'Mcp-Session-Id': 'g5-ui-oturum' });
      res.end(JSON.stringify({ jsonrpc: '2.0', id: rpc.id, result }));
    });
    await new Promise((coz, red) => { sunucu.once('error', red); sunucu.listen(0, '127.0.0.1', coz); });
    k = await Kalfa.ac({ sart: 'kabul', anahtar: GECERLI_ANAHTAR, onay: true, uzak_mcp_test_loopback: true,
      senaryolar: { uzak: { yanitlar: [
        [{ tur: 'arac', ad: 'arac_ara', argumanlar: { sorgu: 'kayıt', sunucu: 'g5ui', yukle: ['mcp__g5ui__kayit_getir'] } }],
        [{ tur: 'arac', ad: 'mcp__g5ui__kayit_getir', argumanlar: { bilet: 42 } }],
        [{ tur: 'arac_sonucu' }],
      ] } } });
    await k.pencere('ana');
    await k.yeniOturum();
    await k.invoke('uzak_mcp_kaydet', { eski_ad: null, form: { ad: 'g5ui', etkin: true,
      uc_url: `http://127.0.0.1:${sunucu.address().port}/mcp`, yetki: { tur: 'yetkisiz' },
      baslatma_zamani_asimi_sn: 5, cagri_zamani_asimi_sn: 5 } });
    await k.invoke('uzak_mcp_baglan', { ad: 'g5ui' });
  });
  after(async () => { await k?.kapat(); if (sunucu) await new Promise(coz => sunucu.close(coz)); });
  it('dinamik keşiften sonra uzak çağrı UI onayını bekler ve gerçek sunucu sonucu görünür', async () => {
    await k.gonder('uzak');
    await k.onaySayfasiniAc();
    assert.equal(cagrilar.length, 0, 'Mühürlü UI onayından önce sunucu çağrılmamalı');
    const sayfa = await k.b.$('[role="dialog"]').getText();
    assert.match(sayfa, /kayit_getir|Kayıt getirin/);
    await k.dugme('Tümünü onayla');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    await k.metniBekle('G5-UI-GERCEK-HTTP-42');
    assert.deepEqual(cagrilar, [{ name: 'kayit_getir', arguments: { bilet: 42 } }]);
    const kayit = JSON.stringify(k.kayit(await k.seciliOturum()));
    assert.match(kayit, /arac_ara/);
    assert.match(kayit, /G5-UI-GERCEK-HTTP-42/);
  });
});
