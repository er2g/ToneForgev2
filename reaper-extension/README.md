# ToneForge REAPER Extension

REAPER için HTTP API sağlayan C++ extension. ToneForge desktop uygulamasının REAPER ile iletişim kurmasını sağlar.

## Özellikler

- 🌐 **HTTP REST API** (localhost:8888)
- 🎛️ **FX Parametre Kontrolü** (fuzzy search ile)
- 🔌 **Plugin Yönetimi** (ekle, sil, listele)
- 🎵 **Transport Kontrolü** (BPM, play, stop)
- 💾 **Proje Yönetimi** (save/load)

## Gereksinimler

### Windows
- Visual Studio 2019 veya üzeri (MSVC)
- CMake 3.15+
- REAPER 6.0+

### macOS
- Xcode Command Line Tools
- CMake 3.15+
- REAPER 6.0+

## Dış Bağımlılıklar

Bu extension şu single-header kütüphaneleri kullanır:

1. **cpp-httplib**: HTTP server ([https://github.com/yhirose/cpp-httplib](https://github.com/yhirose/cpp-httplib))
2. **nlohmann/json**: JSON parsing ([https://github.com/nlohmann/json](https://github.com/nlohmann/json))
3. **REAPER SDK**: REAPER API headers

## Kurulum

### 1. Bağımlılıkları İndir

```bash
cd ToneForge
mkdir -p external && cd external

# REAPER SDK
git clone https://github.com/justinfrankel/reaper-sdk.git

# cpp-httplib
git clone https://github.com/yhirose/cpp-httplib.git

# nlohmann-json
git clone https://github.com/nlohmann/json.git
```

### 2. Build

**Windows (Visual Studio):**
```bash
cd reaper-extension
mkdir build && cd build
cmake .. -G "Visual Studio 16 2019" -A x64
cmake --build . --config Release
```

**macOS:**
```bash
cd reaper-extension
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j4
```

### 3. Install

**Windows:**
```bash
# Manuel
copy build\bin\Release\reaper_toneforge.dll "%APPDATA%\REAPER\UserPlugins\"

# CMake ile otomatik
cmake --install build --config Release
```

**macOS:**
```bash
# Manuel
cp build/bin/reaper_toneforge.dylib ~/Library/Application\ Support/REAPER/UserPlugins/

# CMake ile otomatik
cmake --install build
```

### 4. REAPER'da Aktifleştir

1. REAPER'ı başlat
2. `Extensions > Show Console` menüsünü aç
3. Extension yüklendiğinde şu mesajı göreceksin:
   ```
   ToneForge Extension loaded on port 8888
   ```

## API Dokümantasyonu

### Base URL
```
http://localhost:8888
```

### Endpoints

#### Health Check
```http
GET /ping

Response:
{
  "status": "ok",
  "service": "ToneForge REAPER Extension"
}
```

#### FX Listesi
```http
GET /fx/list?track=0

Response:
{
  "track": 0,
  "fx_count": 2,
  "fx_list": [
    {"index": 0, "name": "VST3: Neural DSP Archetype Gojira (Neural DSP)"},
    {"index": 1, "name": "VST3: FabFilter Pro-Q 3 (FabFilter)"}
  ]
}
```

#### Kurulu Plugin Kataloğu (Parametrelerle)
```http
GET /fx/catalog
GET /fx/catalog?refresh=1  # Cache'i temizleyip yeniden tara

Response:
{
  "count": 2,
  "cache_size": 2,
  "refreshed": false,
  "plugins": [
    {
      "name": "VST3: Neural DSP Archetype Gojira (Neural DSP)",
      "format": "VST3",
      "param_count": 3,
      "params": [
        {"index": 0, "name_raw": "Input", "name_normalized": "input", "default_normalized": 0.5},
        {"index": 1, "name_raw": "Drive", "name_normalized": "drive", "default_normalized": 0.35},
        {"index": 2, "name_raw": "Gate", "name_normalized": "gate", "default_normalized": 0.0}
      ]
    }
  ]
}
```
> Parametre isimleri hem ham hem normalize edilmiş (küçük harf + alfasayısal) olarak döner. Normalize değerler `TrackFX_GetParamNormalized` ile çekilen varsayılan yükleme değerleridir. İlk çağrıda yapılan tarama cache'lenir; `refresh=1` parametresiyle yeniden tarama yapılabilir.

#### FX Parametresi Ayarla
```http
POST /fx/param
Content-Type: application/json

{
  "track": 0,
  "fx": 0,
  "param": "gain",
  "value": 0.8
}

Response:
{
  "success": true,
  "track": 0,
  "fx": 0,
  "param_index": 5,
  "value": 0.8
}
```

#### FX Parametresi Oku
```http
GET /fx/param?track=0&fx=0&param=gain

Response:
{
  "track": 0,
  "fx": 0,
  "param": "gain",
  "param_index": 5,
  "value": 0.75
}
```

#### Plugin Ekle
```http
POST /fx/add
Content-Type: application/json

{
  "track": 0,
  "plugin": "VST3:Neural DSP Archetype Gojira"
}

Response:
{
  "success": true,
  "track": 0,
  "fx_index": 1,
  "fx_name": "VST3: Neural DSP Archetype Gojira (Neural DSP)"
}
```

#### Plugin Sil
```http
DELETE /fx/remove?track=0&fx=1

Response:
{
  "success": true,
  "track": 0,
  "fx": 1
}
```

#### BPM Ayarla
```http
POST /transport/bpm
Content-Type: application/json

{
  "bpm": 140
}

Response:
{
  "success": true,
  "bpm": 140
}
```

#### BPM Oku
```http
GET /transport/bpm

Response:
{
  "bpm": 120.0
}
```

#### Proje Kaydet
```http
POST /project/save
Content-Type: application/json

{
  "name": "my-preset"
}

Response:
{
  "success": true,
  "preset_name": "my-preset",
  "project_path": "C:\\Users\\User\\Documents\\REAPER Projects\\ToneForge.RPP"
}
```

#### Proje Yükle
```http
POST /project/load
Content-Type: application/json

{
  "path": "C:\\Presets\\metallica-tone.RPP"
}

Response:
{
  "success": true,
  "loaded_path": "C:\\Presets\\metallica-tone.RPP"
}
```

## Fuzzy Parameter Search

Extension, parametre isimlerinde "fuzzy search" yapar:

```http
# Bunların hepsi "Master Gain" parametresini bulur:
param: "gain"
param: "master"
param: "mastergain"
param: "Gain"
```

Algoritma:
1. Exact match kontrolü (küçük/büyük harf duyarsız)
2. Substring match (parametre adının içinde arama)
3. Bulunamazsa hata döner ve mevcut parametreleri listeler

## Hata Ayıklama

### Extension yüklenmiyor
```bash
# REAPER console'da log kontrol et
Extensions > Show REAPER resource path in Explorer/Finder
# UserPlugins klasörünü aç, DLL/dylib dosyası orada mı?
```

### HTTP bağlantı hatası
```bash
# Port kullanımda mı kontrol et (Windows)
netstat -ano | findstr :8888

# Port kullanımda mı kontrol et (macOS)
lsof -i :8888
```

### Parametre bulunamıyor
```http
# Önce FX listesini al
GET /fx/list?track=0

# Sonra parametre detaylarını logla (geliştirme modu)
# Extension source'da debug logging aktif et
```

## Güvenlik

- Extension sadece **localhost (127.0.0.1)** dinler
- Dış networke açık değil
- Authentication yok (local-only varsayımı)

## Performans

- Tüm API çağrıları mutex ile korunur
- REAPER audio thread'i bloklanmaz
- HTTP server ayrı thread'de çalışır

## Lisans

MIT License

## Sorun Giderme

**Problem:** DLL yüklenemiyor (Windows)  
**Çözüm:** Visual C++ Redistributable 2019+ kurulu mu kontrol et

**Problem:** "Failed to load plugin" hatası  
**Çözüm:** Plugin adını tam olarak yaz: `VST3:PluginName (Manufacturer)`

**Problem:** BPM değişmiyor  
**Çözüm:** REAPER project tempo mode'u "master tempo" olmalı

## Katkıda Bulunma

Pull request'ler memnuniyetle karşılanır. Büyük değişiklikler için önce issue açın.
