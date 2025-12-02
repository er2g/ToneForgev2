# ToneForge 🎸🤖

**AI-Powered Guitar Tone Assistant for Beginners**

ToneForge, başlangıç seviyesindeki gitaristlerin profesyonel tonlara ulaşmasını sağlayan AI destekli bir uygulama. REAPER DAW ve Neural DSP gibi profesyonel pluginler üzerine inşa edilmiş modern bir mimari.

## 🎯 Özellikler

- 🤖 **AI Ton Asistanı**: "Metallica tonu istiyorum" gibi doğal dille ton talep et
- 🎛️ **Akıllı Parametre Kontrolü**: AI, amp'in gain/bass/treble ayarlarını otomatik optimize eder
- 💾 **Preset Yönetimi**: Favori tonlarını kaydet ve paylaş
- 🎸 **Profesyonel Pluginler**: Neural DSP Archetype serisi desteği
- 🧭 **Plugin Keşfi**: Kurulu VST/VST3/AU eklentilerini ve varsayılan parametrelerini kataloglar
- 🚀 **Offline Çalışır**: Tüm işlemler local'de gerçekleşir (sadece AI sorguları için internet)

## 🏗️ Mimari

```
┌─────────────────────────┐
│   ToneForge Desktop     │  <- Tauri (Rust + React)
│   Modern UI + Chat      │
└───────────┬─────────────┘
            │ HTTP REST API (localhost:8888)
┌───────────▼─────────────┐
│  REAPER Extension (C++) │  <- reaper_toneforge.dll
│  - FX Control           │
│  - Preset Manager       │
│  - Transport Control    │
└───────────┬─────────────┘
            │ REAPER Native API
┌───────────▼─────────────┐
│   REAPER DAW (Gizli)    │  <- Audio Engine
│   + Neural DSP Plugins  │
│   + Diğer VST3'ler      │
└─────────────────────────┘
```

## 📁 Proje Yapısı

```
ToneForge/
├── reaper-extension/      # REAPER C++ Extension (DLL)
│   ├── src/
│   │   ├── main.cpp
│   │   ├── http_server.cpp
│   │   └── fx_controller.cpp
│   ├── CMakeLists.txt
│   └── README.md
│
├── tauri-app/             # Desktop App (Rust + React)
│   ├── src-tauri/         # Rust backend
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── reaper_client.rs
│   │   │   └── gemini_client.rs
│   │   └── Cargo.toml
│   ├── src/               # React frontend
│   │   ├── App.tsx
│   │   └── components/
│   └── package.json
│
├── presets/               # Ton presetleri (.RPP formatında)
│   ├── metallica-master.RPP
│   └── blues-clean.RPP
│
└── docs/                  # Dokümantasyon
    ├── ARCHITECTURE.md
    ├── API.md
    └── SETUP.md
```

## 🚀 Hızlı Başlangıç

### Gereksinimler

- **Windows 10/11** (şimdilik)
- **REAPER 6.0+** (ücretsiz trial yeterli)
- **Neural DSP Archetype Gojira** (veya başka Neural DSP plugin)
- **Rust** (1.70+)
- **Node.js** (18+)
- **CMake** (3.15+) ve MSVC (Visual Studio 2019+)

### Kurulum

1. **REAPER Extension'ı Build Et:**
```bash
cd reaper-extension
mkdir build && cd build
cmake ..
cmake --build . --config Release
```

2. **Extension'ı REAPER'a Kur:**
```bash
copy Release\reaper_toneforge.dll "%APPDATA%\REAPER\UserPlugins\"
```

3. **Tauri App'i Çalıştır:**
```bash
cd tauri-app
npm install
npm run tauri dev
```

4. **REAPER'ı Başlat:**
```bash
# Extension otomatik yüklenecek (Extensions > Show Console'da kontrol et)
```

## 🎮 Kullanım

1. ToneForge uygulamasını aç
2. Chat kutusuna istediğin tonu yaz: *"Heavy metal tonu, sıkı distortion"*
3. AI, REAPER'da otomatik olarak ton zincirini kurar
4. Gitarını bağla ve çal!

## EQ Matcher (Yeni)

ToneForge uygulamasına Next-Level EQ Match motoru entegre edildi. Artık aynı pencerede ikinci bir sekme üzerinden referans miks frekans spektrumunu analiz edip kendi kaydınızı otomatik olarak eşleyebilirsiniz.

### Nasıl kullanılır?

1. Uygulama açıldığında üstteki **Tone Assistant / EQ Matcher** sekmelerinden **EQ Matcher**'ı seçin.
2. Referans miks ve kendi kaydınızı dosya aç diyalogu ile yükleyin (WAV/MP3/FLAC vb. desteklenir).
3. Analiz ekranında her iki profilin spektrumunu inceleyip Match Settings panelinden intensity, smoothing ve psychoacoustic gibi parametreleri ayarlayın.
4. **Calculate EQ Match** ile otomatik eşleme bandlarını oluşturun. Sistem her band için dB, Q ve güven skorlarını gösterir.
5. Sonuçtan memnunsanız **Export Settings** diyaloğu ile .RfxChain, .json veya .txt formatlarında dışa aktarın ve REAPER'a import edin.

EQ motoru tamamen local çalışır; herhangi bir dosya buluta yüklenmez. Varsayılan olarak 48 kHz analiz yapılır ve farklı sample-rate dosyalar otomatik olarak yeniden örneklenir.

## 🔧 Geliştirme

### REAPER Extension API

Extension, `localhost:8888` üzerinden REST API sunar:

```http
# Parametre değiştir
POST /fx/param
Content-Type: application/json

{
  "track": 0,
  "fx": 0,
  "param": "gain",
  "value": 0.8
}

# Plugin ekle
POST /fx/add
{
  "track": 0,
  "plugin": "VST3:Neural DSP Archetype Gojira"
}

# Kurulu pluginleri ve varsayılan parametrelerini listele
GET /fx/catalog
GET /fx/catalog?refresh=1  # Cache'i temizleyip yeniden tara

# BPM değiştir
POST /transport/bpm
{
  "bpm": 120
}
```

### Rust Client Kullanımı

```rust
use reaper_client::ReaperClient;

let reaper = ReaperClient::new();
reaper.set_param(0, 0, "gain", 0.8).await?;
reaper.load_plugin(0, "VST3:Neural DSP Archetype Gojira").await?;
```

## 📝 Roadmap

- [x] REAPER Extension HTTP API
- [x] Tauri Desktop App
- [x] Gemini AI entegrasyonu
- [x] Preset yönetimi
- [ ] macOS desteği
- [ ] Plugin otomatik keşfi
- [ ] Preset paylaşım platformu
- [ ] VST3 chunk state save/load (tam parametre kaydı)

## 🤝 Katkıda Bulunma

Pull request'ler kabul edilir! Büyük değişiklikler için önce issue açın.

## 📄 Lisans

MIT License - Detaylar için LICENSE dosyasına bakın.

## 🙏 Teşekkürler

- REAPER SDK
- Tauri Team
- Neural DSP
- Gemini API

---

**Made with 🎸 by er2g**
