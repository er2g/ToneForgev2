# Top-Level Chain Mapper (TL-Chain)

## Amaç

REAPER zincirinde (FX chain) binlerce parametre olduğunda, modeli bu parametrelerin tamamı ve dönüşümleri ile “boğmak” hataya çok açık hale geliyor. Bu branch, **Tier 2’yi (parametre eşleme + uygulama) deterministikleştirerek** AI’ın dikkatini dağıtmadan iletişimi güçlendiren bir katman ekler.

Bu yaklaşımda AI:
- Tonu **tanımlar** (ToneAI çıktısı: `ToneParameters`)
- Sistem ise tonu **uygular** (Chain Mapper + Action Planner: deterministik)

## Tasarım

### 1) Girdi
- `ToneParameters`: amp/effect/reverb/delay için **0..1 normalize**, EQ için **dB**.
- `ReaperSnapshot`: track + plugin listesi + her plugin’in parametreleri (`name`, `index`, `current_value`, `display`, `format_hint`).

### 2) Chain Mapper (Deterministik Eşleme)

1. **Rol seçimi (plugin seçimi)**:
   - Amp / EQ / Gate / Delay / Reverb vb. için plugin adından **keyword + skor** ile en olası plugin seçilir.
2. **Parametre seçimi**:
   - Tone key → param name eşlemesi **synonym + normalize edilmiş fuzzy match** ile yapılır.
3. **Dönüşüm ve güvenlik**:
   - 0..1 dışı değerler clamp’lenir ve uyarı üretir.
   - EQ (dB) yalnızca desteklenen EQ adapter’ında normalize değere çevrilir (ilk aşama: ReaEQ odaklı).
4. **Aksiyon planı**:
   - Gerekirse `EnablePlugin` önce gelir.
   - `SetParameter` aksiyonları tekilleştirilir (aynı parametreye son değer).

### 3) Uygulama (Action Planner)
Uygulama katmanı:
- Aksiyonları sıralar, tekilleştirir, clamp uygular.
- Her `SetParameter` için (opsiyonel) readback/verify yapılabilir (bu branch’ta temel seviyede tutulur).

## Hedeflenen Kazanç
- AI prompt’u “binlerce parametre” yerine **tone hedefi** ile sınırlanır.
- Parametre eşleme hataları (yanlış parametre, yanlış index, yanlış dönüşüm) deterministik katmanda yakalanır.
- Aynı tone request daha tekrar edilebilir ve debug edilebilir olur (aksiyon log’u deterministik).

## Durum / Sınırlar
- Bu ilk iterasyonda EQ mapping daha sınırlı (ReaEQ için band yaklaşımı + basit dB→normalize).
- Daha ileri iterasyonlar: plugin adapter registry (NeuralDSP / FabFilter vb.), param-range çözümü (format_hint ile solver) ve post-apply doğrulama genişletmesi.

