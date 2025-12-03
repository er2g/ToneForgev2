# İki Katmanlı AI Ton Mühendisi Sistemi

ToneForge artık **iki katmanlı bir AI sistemi** kullanıyor:

## 🏗️ Mimari

```
┌─────────────────────────────────────────────────────────┐
│                    Kullanıcı Mesajı                     │
│         "Chuck Schuldiner Symbolic tone istiyorum"      │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│          🔍 KATMAN 1: Ton Araştırma AI'ı                │
│                 (tone_researcher.rs)                     │
├─────────────────────────────────────────────────────────┤
│ • Kullanıcı mesajını analiz eder                        │
│ • Ton talebi detect eder (sanatçı, albüm, şarkı)        │
│ • İnternette araştırma yapar:                           │
│   - Equipboard (artist gear)                            │
│   - DuckDuckGo (web search)                             │
│   - YouTube (metadata & descriptions)                   │
│ • Detaylı ton bilgisi toplar:                           │
│   - Amplifikatör ayarları (gain, bass, mid, treble)    │
│   - Efekt zinciri (distortion, delay, reverb, etc.)    │
│   - Ekipman listesi (pedallar, gitarlar)               │
│   - Teknikler (palm muting, tuning, etc.)              │
│ • Sonuçları cache'ler (7 gün TTL)                       │
└──────────────────────┬──────────────────────────────────┘
                       │
                       │ Research Context
                       ▼
┌─────────────────────────────────────────────────────────┐
│         🎛️ KATMAN 2: Ton Uygulama AI'ı                  │
│              (mevcut AI sistemi)                         │
├─────────────────────────────────────────────────────────┤
│ • Araştırma sonuçlarını alır                            │
│ • Bilgisayardaki mevcut pluginleri tarar                │
│ • Ton bilgisine göre en uygun parametreleri belirler    │
│ • Plugin parametrelerini ayarlar:                       │
│   - SetParam (parametre değiştirme)                     │
│   - ToggleFx (plugin aç/kapa)                           │
│   - LoadPlugin (yeni plugin ekleme)                     │
│ • AI Engine optimizasyonları uygular:                   │
│   - Conflict detection                                  │
│   - Deduplication                                       │
│   - Safety validation                                   │
│   - Relationship suggestions                            │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│              Plugin Parametreleri Uygulandı             │
│                  Ton Oluşturuldu ✅                      │
└─────────────────────────────────────────────────────────┘
```

## 📋 Özellikler

### Katman 1: Ton Araştırma AI'ı

#### Otomatik Ton Talebi Algılama
```rust
// Mesajdan ton talebini otomatik algılar
detect_tone_request("Chuck Schuldiner Symbolic tone")
→ ToneRequest {
    artist: "Chuck Schuldiner",
    album: "Symbolic",
    genre: "death metal",
    instrument: "guitar"
}
```

#### Çoklu Kaynak Araştırması
1. **Equipboard**: Sanatçının kullandığı ekipman
2. **Web Search**: Forum tartışmaları, ton ayarları
3. **YouTube**: Video açıklamaları, tutorial bilgileri

#### Detaylı Bilgi Toplama
```rust
ToneInfo {
    description: "Death tonu, yüksek gain, scooped mids...",
    amp_settings: {
        "gain": "8/10",
        "bass": "6/10",
        "mid": "3/10",
        "treble": "7/10",
        "presence": "8/10"
    },
    effects_chain: [
        Effect {
            name: "Distortion",
            effect_type: "Distortion",
            parameters: {...}
        },
        Effect {
            name: "Delay",
            effect_type: "Delay",
            parameters: {...}
        }
    ],
    equipment: [
        "Boss HM-2",
        "Marshall JCM800",
        "ESP Guitar"
    ],
    techniques: [
        "palm muting",
        "down picking",
        "drop tuning"
    ],
    confidence: 0.85
}
```

#### Akıllı Önbellekleme
- **7 gün TTL**: Aynı ton sorguları cache'den gelir
- **Hız**: İlk araştırma ~5 saniye, sonrakiler <1ms
- **Cache key**: Artist + Album + Song + Genre

### Katman 2: Ton Uygulama AI'ı

#### Araştırma Sonuçlarını Kullanma
```
=== TONE RESEARCH RESULTS ===

Description:
Death metal tone, high gain with scooped mids...

Equipment:
  - Boss HM-2
  - Marshall JCM800
  - ESP Guitar

Amp Settings:
  - gain: 8/10
  - bass: 6/10
  - mid: 3/10
  - treble: 7/10

Effects Chain:
  - Distortion (High Gain)
  - Delay (Slap Back)

Playing Techniques:
  - palm muting
  - down picking

Confidence: 85%

=== END RESEARCH RESULTS ===
```

AI bu bilgiyi kullanarak:
1. Mevcut pluginlerden en uygunları seçer
2. Parametreleri araştırma sonuçlarına göre ayarlar
3. Gerekli efektleri sıraya göre ekler

## 🎯 Kullanım Örnekleri

### Örnek 1: Klasik Metal Tonu
```
Kullanıcı: "Metallica Master of Puppets tone istiyorum"

Katman 1 (Araştırma):
  ✓ Equipboard'dan James Hetfield gear bilgisi
  ✓ Mesa Boogie Mark IIC+ amp ayarları
  ✓ Tube Screamer boost settings
  ✓ Down picking tekniği

Katman 2 (Uygulama):
  ✓ Neural DSP Archetype Petrucci loaded
  ✓ Gain: 0.75 (7.5/10)
  ✓ Bass: 0.6, Mid: 0.5, Treble: 0.7
  ✓ Tight gate enabled
  ✓ Boost pedal added before amp
```

### Örnek 2: Death Metal Tonu
```
Kullanıcı: "Chuck Schuldiner Symbolic tonu"

Katman 1 (Araştırma):
  ✓ Boss HM-2 Heavy Metal pedal
  ✓ Scooped mids (mid: 3/10)
  ✓ High gain + high treble
  ✓ Palm muting + drop tuning

Katman 2 (Uygulama):
  ✓ HM-2 style distortion plugin
  ✓ Gain: 0.9, Mid: 0.3, Treble: 0.8
  ✓ EQ scooped for Swedish death metal
  ✓ Gate threshold adjusted
```

### Örnek 3: Jazz Tonu
```
Kullanıcı: "George Benson jazz guitar tone"

Katman 1 (Araştırma):
  ✓ Clean tone, low gain
  ✓ Chorus + reverb
  ✓ Neck pickup
  ✓ Compressed, smooth attack

Katman 2 (Uygulama):
  ✓ Clean amp sim loaded
  ✓ Gain: 0.2, bass/mid/treble balanced
  ✓ Compressor added (ratio 3:1)
  ✓ Chorus + plate reverb
```

## 🔧 Teknik Detaylar

### Dosya Yapısı
```
tauri-app/src-tauri/src/
├── tone_researcher.rs     # Yeni: Katman 1 AI
│   ├── ToneResearcher     # Ana struct
│   ├── ToneRequest        # Parse edilmiş talep
│   ├── ToneInfo           # Toplanan bilgi
│   └── Effect             # Efekt detayları
│
└── lib.rs                 # Güncellenmiş: Entegrasyon
    ├── SYSTEM_PROMPT      # İki katmanlı sistem açıklaması
    ├── PromptPayload      # + research_context field
    ├── process_chat_message # + tone research logic
    └── build_prompt       # + research context injection
```

### Veri Akışı

```rust
// 1. Kullanıcı mesajı gelir
let message = "Chuck Schuldiner Symbolic tone";

// 2. Ton talebi detect edilir
let tone_request = state.tone_researcher.detect_tone_request(&message);

// 3. İnternetten araştırma yapılır
let tone_info = state.tone_researcher.research_tone(&tone_request).await?;

// 4. Sonuç formatlanır
let research_context = state.tone_researcher.format_for_ai(&tone_info);

// 5. Ana AI'a context olarak verilir
let payload = PromptPayload {
    selected_track: track_idx,
    tracks: tracks_snapshot,
    recent_messages: history,
    research_context: Some(research_context), // 👈 Yeni!
};

// 6. Ana AI prompt'u alır ve uygulamayı yapar
let ai_plan = ai_provider.generate(&prompt).await?;
```

### Cache Mekanizması

```rust
// Cache key generation
fn make_cache_key(request: &ToneRequest) -> String {
    format!(
        "{}_{}_{}_{}",
        artist, album, song, genre
    ).to_lowercase()
}

// Cache storage
struct CachedResult {
    info: ToneInfo,
    timestamp: SystemTime,
}

// TTL: 7 days
const CACHE_TTL_SECS: u64 = 7 * 24 * 60 * 60;
```

## 🚀 Performans

### İlk Araştırma (Cache Miss)
- **Süre**: ~5 saniye
- **İşlemler**:
  - Equipboard search: ~1.5s
  - Web search: ~2s
  - YouTube search: ~1.5s
- **Sonuç**: Detaylı ton bilgisi + cache'e kayıt

### Sonraki Sorgular (Cache Hit)
- **Süre**: <1 milisaniye
- **İşlemler**: Cache'den okuma
- **Sonuç**: Aynı detaylı ton bilgisi

### Rate Limiting
- **Timeout**: Her kaynak için 5 saniye
- **Max Results**: 5 adet sonuç
- **Paralel**: Tüm kaynaklar paralel aranır

## 🔍 Algılama Mantığı

### Ton Talebi Algılama
```rust
// Anahtar kelimeler
let tone_keywords = [
    "tone", "sound", "tonu", "ses", "ayar", "settings",
    "amp", "pedal", "effect", "distortion", "reverb", "delay"
];

// Örnekler
✅ "Metallica tone istiyorum"           → Algılandı
✅ "Chuck Schuldiner Symbolic sound"    → Algılandı
✅ "Jazz guitar tonu nasıl olmalı"      → Algılandı
❌ "Merhaba, nasılsın?"                 → Algılanmadı
❌ "Projeyi aç"                         → Algılanmadı
```

### Sanatçı/Albüm/Şarkı Parse
```rust
// Büyük harflerle başlayan kelimeler → Sanatçı
"Chuck Schuldiner" → artist

// Tırnak içindeki kelimeler → Albüm
"'Symbolic' albümü" → album: "Symbolic"

// Album/song marker'ları
"from Master of Puppets" → album: "Master of Puppets"
"song Enter Sandman" → song: "Enter Sandman"
```

## 📊 Güven Skoru (Confidence)

Araştırma kalitesini ölçer (0.0 - 1.0):

```rust
let mut score = 0.0;
if !description.is_empty()    { score += 0.2; }  // Açıklama var
if !amp_settings.is_empty()   { score += 0.2; }  // Amp ayarları bulundu
if !effects_chain.is_empty()  { score += 0.2; }  // Efektler listelendi
if !equipment.is_empty()      { score += 0.2; }  // Ekipman belirlendi
if !techniques.is_empty()     { score += 0.1; }  // Teknikler var
if !sources.is_empty()        { score += 0.1; }  // Kaynaklar eklendi
```

**Yorumlama**:
- **0.8-1.0**: Mükemmel (tüm detaylar bulundu)
- **0.6-0.8**: İyi (çoğu bilgi var)
- **0.4-0.6**: Orta (bazı bilgiler eksik)
- **0.0-0.4**: Zayıf (az bilgi bulundu)

## 🎨 SYSTEM_PROMPT Güncellemesi

AI'a yeni mimari açıklandı:

```
=== TWO-LAYER AI SYSTEM ===

You are the SECOND AI layer in a two-layer system:

🔍 FIRST LAYER (Tone Research AI):
- When users request specific tones (e.g., "Chuck Schuldiner Symbolic tone")
- Automatically searches the internet (Equipboard, forums, YouTube, etc.)
- Gathers detailed information: equipment, amp settings, effects chain, techniques
- Provides you with a "TONE RESEARCH RESULTS" section if available

🎛️ SECOND LAYER (You - Tone Implementation AI):
- You receive the research results from the first AI layer
- Your job is to IMPLEMENT those findings using available plugins
- Match the described tone as closely as possible with current plugin parameters
- If research results are available, USE THEM as your primary reference
```

## 🧪 Test Senaryoları

### Test 1: Metal Tonu
```bash
Input: "Metallica Master of Puppets tone"
Expected:
  - Equipboard'dan James Hetfield gear
  - Mesa Boogie amp settings
  - High gain + mid scoop
  - Tube Screamer boost
```

### Test 2: Jazz Tonu
```bash
Input: "George Benson jazz guitar tone"
Expected:
  - Clean amp
  - Low gain, balanced EQ
  - Chorus + reverb
  - Compression
```

### Test 3: Death Metal
```bash
Input: "Swedish death metal tone"
Expected:
  - Boss HM-2 style
  - Extreme mid scoop
  - High gain + treble
  - Gate + tight low end
```

### Test 4: Cache Testi
```bash
# İlk çağrı
Input: "Metallica tone"
Time: ~5 seconds
Cache: MISS

# İkinci çağrı (aynı ton)
Input: "Metallica tone"
Time: <1ms
Cache: HIT ✅
```

## 🔮 Gelecek Geliştirmeler

### Kısa Vadede
- [ ] Daha fazla ton kaynağı (Ultimate Guitar, Reddit API)
- [ ] NLP tabanlı daha iyi artist/album parsing
- [ ] Preset database entegrasyonu
- [ ] Ton karşılaştırma (reference audio matching)

### Uzun Vadede
- [ ] Makine öğrenmesi ile ton tanıma
- [ ] Kullanıcı ton kütüphanesi
- [ ] Topluluk ton paylaşımı
- [ ] A/B ton karşılaştırması

## 📝 Notlar

### Önemli Noktalar
1. **İlk katman otomatik**: Kullanıcı "search yap" demesine gerek yok
2. **Cache akıllı**: Gereksiz API çağrısı yapmaz
3. **İkinci katman bağımsız**: Research başarısız olsa da normal çalışır
4. **Paralel arama**: Tüm kaynaklar aynı anda aranır (performans)

### Sınırlamalar
1. **İnternet gerekli**: İlk katman offline çalışmaz
2. **İngilizce ağırlıklı**: Türkçe ton bilgisi sınırlı
3. **Cache TTL**: 7 gün sonra yeniden arama gerekir
4. **Rate limiting**: Her kaynak 5 saniye timeout

---

## 🎯 Özet

ToneForge artık **iki akıllı AI katmanı** kullanarak:
1. 🔍 **İnternetten otomatik ton araştırması** yapar
2. 🎛️ **Bulduğu bilgileri mevcut pluginlere uygular**

Kullanıcı sadece "Metallica tone istiyorum" der, sistem otomatik olarak:
- İnternetten araştırır ✅
- Detayları bulur ✅
- Pluginlere uygular ✅
- Tonu oluşturur ✅

**Hepsi otomatik, kullanıcı hiçbir şey yapmaz!** 🚀
