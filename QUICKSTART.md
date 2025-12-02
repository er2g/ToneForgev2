# ToneForge - Hızlı Başlangıç

## 🎸 Nedir Bu?

ToneForge, gitaristlerin AI ile konuşarak profesyonel tonlara ulaşmasını sağlayan bir masaüstü uygulaması. "Metallica tonu istiyorum" dediğinde, AI otomatik olarak amplifier ayarlarını yapıyor.

## ⚡ 5 Dakikada Başla

### 1. Gereksiniml eri Kur (İlk Seferlik)

**Windows 10/11 için:**
1. Visual Studio 2019+ (Community): https://visualstudio.microsoft.com/downloads/
2. CMake: https://cmake.org/download/
3. Git: https://git-scm.com/download/win
4. Node.js 18+: https://nodejs.org/
5. Rust: https://www.rust-lang.org/tools/install
6. REAPER: https://www.reaper.fm/download.php (60 gün trial)
7. Neural DSP plugin (opsiyonel): https://neuraldsp.com/

### 2. Projeyi İndir ve Kur

```cmd
# Terminal aç (CMD veya PowerShell)
cd Desktop
git clone [GITHUB_URL] ToneForge
cd ToneForge

# Otomatik kurulum
setup.bat
```

Script her şeyi otomatik yapacak (~5 dakika).

### 3. Çalıştır

```cmd
# REAPER'ı başlat (arka planda çalışsın)
# Extensions > Show Console'da "ToneForge Extension loaded" görmeli

# Yeni terminal:
cd ToneForge\tauri-app
npm run tauri dev
```

### 4. Gemini API Key Al (Ücretsiz)

1. https://makersuite.google.com/app/apikey adresine git
2. "Create API Key" bas
3. Key'i kopyala
4. ToneForge açıldığında yapıştır

### 5. Kullan!

```
Sen: "I want a heavy metal tone"
AI: *Neural DSP Gojira yükler, gain/bass/treble ayarlar*

Sen: "Boost the gain to 80%"
AI: *Gain parametresini bulup %80'e çeker*

Sen: "Add more warmth"
AI: *Bass ve mid parametrelerini ayarlar*
```

## 📁 Proje Yapısı

```
ToneForge/
├── reaper-extension/     # REAPER C++ Extension (HTTP API)
│   ├── src/
│   │   └── reaper_toneforge.cpp
│   └── CMakeLists.txt
│
├── tauri-app/            # Desktop App (Rust + React)
│   ├── src-tauri/        # Rust backend
│   └── src/              # React frontend
│
├── presets/              # Hazır ton presetleri
├── docs/                 # Detaylı dokümantasyon
├── setup.bat             # Otomatik kurulum
└── build.bat             # Production build
```

## 🔧 Sorun mu Var?

### "REAPER Extension yüklenmiyor"
```cmd
# Kontrol et:
dir "%APPDATA%\REAPER\UserPlugins\reaper_toneforge.dll"

# Yoksa manuel kopyala:
copy reaper-extension\build\bin\Release\reaper_toneforge.dll "%APPDATA%\REAPER\UserPlugins\"
```

### "Port 8888 kullanımda"
```cmd
# Kim kullanıyor bak:
netstat -ano | findstr :8888

# Öldür:
taskkill /PID [PROCESS_ID] /F
```

### "Plugin bulunamıyor"
- REAPER'da `Options > Preferences > Plug-ins > VST3` yolunu kontrol et
- Plugin'i REAPER'da manuel ekleyerek test et
- Tam isim kullan: `"VST3:Neural DSP Archetype Gojira"`

## 📚 Daha Fazla Bilgi

- **Detaylı Kurulum:** `docs/SETUP.md`
- **API Dokümantasyonu:** `reaper-extension/README.md`
- **Mimari:** `docs/ARCHITECTURE.md`

## 🎯 Hızlı Testler

### Test 1: REAPER Bağlantısı
```bash
# Terminal'de:
curl http://localhost:8888/ping

# Sonuç: {"status":"ok","service":"ToneForge REAPER Extension"}
```

### Test 2: Plugin Ekle
```bash
curl -X POST http://localhost:8888/fx/add \
  -H "Content-Type: application/json" \
  -d '{"track":0, "plugin":"VST3:Neural DSP Archetype Gojira"}'
```

### Test 3: Gain Ayarla
```bash
curl -X POST http://localhost:8888/fx/param \
  -H "Content-Type: application/json" \
  -d '{"track":0, "fx":0, "param":"gain", "value":0.8}'
```

## 🚀 Production Build

Exe dosyası oluşturmak için:

```cmd
build.bat
```

Output: `tauri-app\src-tauri\target\release\toneforge.exe`

## ❤️ Teşekkürler

- REAPER SDK
- Tauri Team
- Neural DSP
- Google Gemini

---

**Keyifli müzikler! 🎸**

GitHub Issues: [PROJE_URL]/issues
