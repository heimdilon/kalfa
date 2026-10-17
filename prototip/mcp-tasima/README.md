# MCP taşıma ön doğrulaması (G2)

1.1a-2 planının SDK bağlantı deneyi; ürün workspace'inin parçası değildir.
`rmcp =3.5.1`, üretim dependency'sinde yalnız client, dev'de server.
Dev server özelliği transitif olarak transport-async-rw açsa da istemci
bu taşıma yolunu kullanmaz: `SinirliTasima`, `Transport<RoleClient>` uygular.

```sh
cargo test --manifest-path prototip/mcp-tasima/Cargo.toml --locked -- --test-threads=1
cargo clippy --manifest-path prototip/mcp-tasima/Cargo.toml --all-targets --locked -- -D warnings
cargo fmt --manifest-path prototip/mcp-tasima/Cargo.toml -- --check
```

Yedi test: 4 MiB satır sınırı, parçalı UTF-8/EOF, 64 ileti/taşma, bin liste
bildirimini birleştirme, bozuk JSON hatası, eski ve yeni MCP dönemleri.
İki dönem testi pozitif TTL'ye rağmen SDK yanıt cache'inin kapalı olduğunu,
ping yanıtını ve roots/sampling reddini de ölçer. Sunucu sentetiktir;
Tokio duplex kullanır, dosya/ağ/gerçek süreç oluşturmaz.

Bu, bütün ürünün bellek veya kapanış güvencesi değildir. Kuyruklar ileti
sayısını sınırlar; typed JSON ve kopyalar ek bellek tutar. Görevler close
ve drop'ta abort edilir; ürün yöneticisinin join/kill/son-denetim protokolü
bu deneyde yoktur. stderr, sır kasası, çağrı iptali, UI ve gerçek OS yalıtımı
planın sonraki görevlerinde sınanır. Ürüne olduğu gibi kopyalayıp bu
kapıların tamamlandığı söylenmez.

Rapor: ../../docs/superpowers/bulgular/2026-10-15-kalfa-11a-2-on-dogrulama.md.
Plan: ../../docs/superpowers/plans/2026-10-15-kalfa-11a-2-mcp.md.