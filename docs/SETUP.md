# ToneForge Kurulum Kılavuzu

## Ön Gereksinimler

### 1. Windows Gereksinimleri
- **Windows 10/11** (64-bit)
- **Visual Studio 2019 veya üzeri** (Community edition yeterli)
  - Kurulumda "Desktop development with C++" seçeneğini işaretle
  - İndirme: https://visualstudio.microsoft.com/downloads/
- **CMake 3.15+**
  - İndirme: https://cmake.org/download/
  - Kurulumda "Add CMake to system PATH" seçeneğini işaretle
- **Git**
  - İndirme: https://git-scm.com/download/win
- **Node.js 18+**
  - İndirme: https://nodejs.org/
- **Rust** (Tauri için)
  - İndirme: https://www.rust-lang.org/tools/install
  - PowerShell'de çalıştır: `rustup-init.exe`

### 2. REAPER ve Pluginler
- **REAPER 6.0+**
  - İndirme: https://www.reaper.fm/download.php
  - 60 günlük trial yeterli
- **Neural DSP Archetype Gojira** (veya başka Neural DSP plugin)
  - İndirme: https://neuraldsp.com/
  - Gojira'nın 14 günlük trial'ı var
- Alternatif: Herhangi bir VST3 amp simulator (TSE808, LePou, vb.)

### 3. Gemini API Key
- Google AI Studio: https://makersuite.google.com/app/apikey
- **ÜCRETSİZ** (aylık limit dahilinde)
- Kayıt ol ve "Create API Key" butonuna bas

---

## Otomatik Kurulum (Kolay Yol)

### Windows

1. **Projeyi indir**
   ```cmd
   cd C:\Users\[KULLANICI_ADIN]\Desktop
   git clone [PROJE_URL] ToneForge
   cd ToneForge
   ```

2. **Setup script'ini çalıştır**
   ```cmd
   setup.bat
   ```

3. Script şunları otomatik yapar:
   - ✅ Bağımlılıkları indirir (REAPER SDK, http server, json parser)
   - ✅ REAPER extension'ı build eder
   - ✅ Extension'ı REAPER'a kurar
   - ✅ Tauri app dependencies kurar

4. **REAPER'ı başlat**
   - Extensions > Show Console açıp extension'ın yüklendiğini kontrol et
   - Görmeli: `ToneForge Extension loaded on port 8888`

5. **ToneForge'u başlat**
   ```cmd
   cd tauri-app
   npm run tauri dev
   ```

6. **API Key'ini gir** ve kullanmaya başla!

---

## Manuel Kurulum (İleri Seviye)

### Adım 1: External Dependencies

```bash
mkdir external && cd external

# REAPER SDK
git clone https://github.com/justinfrankel/reaper-sdk.git

# HTTP Server (single-header)
git clone https://github.com/yhirose/cpp-httplib.git

# JSON Parser (single-header)
git clone https://github.com/nlohmann/json.git

cd ..
```

### Adım 2: REAPER Extension Build

```bash
cd reaper-extension
mkdir build && cd build

# CMake configure
cmake .. -G "Visual Studio 16 2019" -A x64

# Build (Release mode)
cmake --build . --config Release

cd ../..
```

**Output:** `reaper-extension/build/bin/Release/reaper_toneforge.dll`

### Adım 3: Extension'ı REAPER'a Kur

**Manuel kopya:**
```cmd
copy reaper-extension\build\bin\Release\reaper_toneforge.dll "%APPDATA%\REAPER\UserPlugins\"
```

**Veya CMake install:**
```cmd
cmake --install reaper-extension/build --config Release
```

**Kontrol:**
- REAPER'ı başlat
- `Extensions > Show Console` aç
- Log'da "ToneForge Extension loaded on port 8888" görmeli

### Adım 4: Tauri App Setup

```bash
cd tauri-app

# Dependencies kur
npm install

# Development mode (hot-reload ile)
npm run tauri dev

# Production build (exe dosyası)
npm run tauri build
```

**Output (build):** `tauri-app/src-tauri/target/release/toneforge.exe`

---

## Sorun Giderme

### REAPER Extension Yüklenmiyor

**Belirtiler:**
- REAPER console'da extension göremiyorum
- ToneForge app "REAPER Disconnected" diyor

**Çözümler:**

1. **DLL dosyası doğru klasörde mi?**
   ```cmd
   dir "%APPDATA%\REAPER\UserPlugins\reaper_toneforge.dll"
   ```
   - Yoksa manuel kopyala

2. **Visual C++ Redistributable kurulu mu?**
   - İndirme: https://aka.ms/vs/17/release/vc_redist.x64.exe
   - Kur ve REAPER'ı yeniden başlat

3. **REAPER'ı yönetici modunda çalıştır**
   - Sağ tık > "Run as administrator"

4. **Extension log'larını kontrol et**
   - REAPER Console'da hata var mı?
   - `%APPDATA%\REAPER\reaper.ini` dosyasında extension path doğru mu?

### Port 8888 Kullanımda

**Hata:** "Address already in use"

```cmd
# Hangi program kullanıyor bak
netstat -ano | findstr :8888

# Process'i öldür (PID numarasını kullan)
taskkill /PID [PROCESS_ID] /F
```

### Tauri Build Hatası

**"Rust toolchain not found":**
```bash
# Rust'ı kur
https://www.rust-lang.org/tools/install

# Kontrol et
rustc --version
cargo --version
```

**"webkit2gtk not found" (Linux):**
```bash
# Ubuntu/Debian
sudo apt install libwebkit2gtk-4.0-dev

# Arch Linux
sudo pacman -S webkit2gtk
```

### Plugin Bulunamıyor

**Gemini diyor: "Failed to load plugin"**

**Çözümler:**

1. **Plugin tam adını kullan**
   ```
   ❌ "Neural DSP"
   ✅ "VST3:Neural DSP Archetype Gojira"
   ```

2. **REAPER'da plugin yolunu kontrol et**
   - `Options > Preferences > Plug-ins > VST3`
   - Neural DSP path'i ekli mi?
   - Scan/Rescan yap

3. **Manuel test**
   - REAPER'da plugin'i elle ekle
   - Çalışıyor mu? (çalışmıyorsa plugin sorunu)

---

## Performans İyileştirmeleri

### 1. REAPER Audio Settings

- `Options > Preferences > Audio > Device`
  - **Buffer size:** 256 samples (latency: ~6ms)
  - **ASIO driver** kullan (mümkünse)

### 2. Gemini API Caching

Gemini'ye aynı komutları tekrar göndermemek için:

```rust
// Tauri backend'de cache ekle
use std::collections::HashMap;

struct CommandCache {
    cache: HashMap<String, AICommand>,
}
```

### 3. Production Build

Development mode yerine production:

```bash
cd tauri-app
npm run tauri build
```

- ~10MB exe dosyası
- 5x daha hızlı başlangıç
- GPU optimization

---

## Kullanım Senaryoları

### Senaryo 1: Sıfırdan Metal Tonu

1. REAPER'da boş track aç
2. ToneForge'a yaz: **"I want a Metallica-style tone"**
3. AI otomatik yükler: Neural DSP Gojira
4. Gain/Bass/Treble'ı ayarlar
5. Gitarını bağla ve çal!

### Senaryo 2: Mevcut Tonu İyileştir

1. REAPER'da Neural DSP zaten yüklü
2. ToneForge: **"Boost the gain to 80%"**
3. AI parametre bulur ve değiştirir
4. Sonuç: Anında daha aggressive ton

### Senaryo 3: Preset Kaydetme

1. Mükemmel tonu bul
2. ToneForge'da "💾 Save Preset" bas
3. İsim ver: "Metallica Master"
4. Sonraki sefer: REAPER'da preset'i seç

---

## İleri Seviye: VST3 Chunk Save

Şu anki versiyon sadece plugin listesini kaydediyor. Parametreleri de kaydetmek için:

### C++ Extension Güncellemesi

```cpp
// reaper_toneforge.cpp'ye ekle

g_server.Post("/fx/get_state", [](const httplib::Request& req, httplib::Response& res) {
    // VST3 chunk'ı binary olarak al
    char chunk[65536];
    bool success = TrackFX_GetNamedConfigParm(track, fx_idx, "chunk", chunk, 65536);
    
    // Base64 encode et ve JSON'a koy
    res.set_content(json({{"chunk", base64_encode(chunk)}}).dump(), "application/json");
});
```

Bu özellik gelecek versiyonlarda eklenecek.

---

## Yardım ve Destek

### Loglar

**REAPER:**
- `Extensions > Show Console`

**Tauri:**
- Development: Terminal'de direkt görünür
- Production: `%APPDATA%\toneforge\logs\`

**Extension:**
- Windows Event Viewer (nadiren gerekir)

### GitHub Issues

Sorun bulursan:
1. Log dosyalarını ekle
2. Adım adım nasıl reproduce edileceğini yaz
3. Windows/REAPER/Plugin versiyonlarını belirt

---

## Katkıda Bulunma

Pull request'ler memnuniyetle karşılanır!

**Öncelikli ToDo:**
- [ ] macOS desteği
- [ ] VST3 chunk save/load
- [ ] Plugin auto-discovery (sistemdeki tüm VST3'leri tara)
- [ ] Preset paylaşım platformu

---

**Keyifli tonlar! 🎸**
