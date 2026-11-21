# Vitrin

GitHub projelerinizi (kişisel + organizasyon) vitrin kartları halinde gösteren Rust masaüstü uygulaması.

## Çalıştırma
```
cargo run --release
```
İlk açılışta GitHub Personal Access Token girin (classic: `repo` + `read:org`, veya fine-grained: Metadata/Contents okuma).

## Özellikler
- Kart ızgarası, arama (ad/açıklama/topic), dil filtresi, sıralama
- Detay paneli: istatistikler, dil dağılım çubuğu, GitHub'da aç
- Ağ istekleri ayrı thread'lerde → arayüz donmaz

## Platform notları
- Linux: `libxkbcommon`, `libwayland`/`libx11` geliştirme paketleri gerekebilir
- Token şu an `config_dir()/vitrin/token` dosyasında düz metin saklanır; üretimde `keyring` crate'i önerilir

## GitHub Actions
- `ci.yml`: her push/PR'da Linux, macOS, Windows üzerinde clippy + release build
- `release.yml`: `v*` etiketi atınca 4 hedef için paket üretir ve Release'e yükler:
  ```
  git tag v1.0.0 && git push origin v1.0.0
  ```
  Hedefler: Linux x86_64, Windows x86_64, macOS Apple Silicon, macOS Intel.

## Lisans
MIT
