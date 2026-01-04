# Online Test Run (Gemini 2.5 Pro)

- date_utc: 2026-01-04T20:38:40Z
- command: cargo run -q --bin gemini_chain_test

## Raw Output

```text
=== Gemini Chain Test Report ===

- Delay bypassed section [confusing_delay_section] => PASS (vertex-gemini)
  tone: A tight, high-gain modern rhythm tone with a subtle slapback delay for added depth without sacrificing clarity.
  engineer_score: 100 (warnings: 0)
  mapping_warnings: 1
    - Section gate 'Delay Bypass' appears inactive; inserting toggle before setting 'Delay Mix'
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: set fx 0 param Bass -> 0.550
  log: set fx 0 param Treble -> 0.700
  log: set fx 0 param Gain -> 0.800

- Reverb bypassed section [bypassed_reverb] => PASS (vertex-gemini)
  tone: A very subtle, small room reverb designed to add a sense of space while maintaining attack clarity and preventing wash-o…
  engineer_score: 100 (warnings: 0)
  mapping_warnings: 1
    - Section gate 'Reverb Bypass' appears inactive; inserting toggle before setting 'Room Size'
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: set fx 1 param Reverb Bypass -> 0.000
  log: set fx 1 param Room Size -> 0.200
  log: set fx 1 param Decay -> 0.250

- Gate plugin disabled [disabled_gate] => PASS (vertex-gemini)
  tone: Very tight high-gain metal rhythm tone with a fast and aggressive noise gate, ideal for percussive riffing.
  engineer_score: 100 (warnings: 0)
  mapping_warnings: 1
    - Unmapped amp param 'master' for plugin 'VST3: Neural DSP Archetype'
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: enabled fx 1
  log: set fx 0 param Treble -> 0.750
  log: set fx 0 param Bass -> 0.450

- Gate section disabled [gate_enable_off] => PASS (vertex-gemini)
  tone: A very tight, aggressive high-gain metal rhythm tone, featuring a fast-acting noise gate for precise, staccato riffing.
  engineer_score: 100 (warnings: 0)
  mapping_warnings: 1
    - Section gate 'Gate Enable' appears inactive; inserting toggle before setting 'Attack'
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: set fx 0 param Presence -> 0.800
  log: set fx 0 param Bass -> 0.700
  log: set fx 0 param Gain -> 0.850

- Reverb missing (should load) [missing_reverb] => PASS (vertex-gemini)
  tone: A clean, clear tone with a subtle small room reverb for a touch of ambience and space, designed to maintain clarity.
  engineer_score: 95 (warnings: 1)
    - reverb mix too high (0.150 > 0.120)
  mapping_warnings: 1
    - clamped reverb.mix from 0.150 to 0.120
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: loaded 'ReaVerbate (Cockos)' slot 2
  log: set fx 0 param Bass -> 0.500
  log: set fx 0 param Presence -> 0.500

- EQ bypassed section [bypassed_eq] => PASS (vertex-gemini)
  tone: A tight, modern high-gain tone with a focused low-end and enhanced presence. The EQ is specifically tailored to reduce m…
  engineer_score: 100 (warnings: 0)
  mapping_warnings: 1
    - Section gate 'EQ Bypass' appears inactive; inserting toggle before setting 'Band 1 Freq'
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: set fx 0 param Presence -> 0.650
  log: set fx 0 param Mid -> 0.400
  log: set fx 0 param Treble -> 0.700

- Kitchen sink contradictions [kitchen_sink] => PASS (vertex-gemini)
  tone: Extreme modern djent tone with a very tight gate, scooped low-mids, and enhanced attack, designed for clarity and articu…
  engineer_score: 100 (warnings: 0)
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: loaded 'ReaGate (Cockos)' slot 2
  log: loaded 'ReaEQ (Cockos)' slot 3
  log: enabled fx 0

- Dual delay prefer ReaDelay [dual_delay_prefer_readelay] => PASS (vertex-gemini+repair)
  tone: A subtle slapback delay effect, characterized by a short delay time, minimal repeats, and a low mix level to create a ti…
  engineer_score: 95 (warnings: 1)
    - delay mix too high (0.250 > 0.200)
  mapping_warnings: 2
    - Section gate 'Delay Bypass' appears inactive; inserting toggle before setting 'Delay Mix'
    - clamped delay.mix from 0.250 to 0.200
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: set fx 2 param Delay Bypass -> 0.000
  log: set fx 2 param Delay Mix -> 0.200
  log: set fx 2 param Delay Feedback -> 0.100

- Shoegaze wall (niche) [shoegaze_wall] => PASS (vertex-gemini+repair)
  tone: A dense, swirling shoegaze wall of sound inspired by My Bloody Valentine. Features a deep, warbly chorus, rhythmic dotte…
  engineer_score: 100 (warnings: 0)
  mapping_warnings: 3
    - Section gate 'Chorus Bypass' appears inactive; inserting toggle before setting 'Depth'
    - Section gate 'Reverb Bypass' appears inactive; inserting toggle before setting 'Pre-delay'
    - Section gate 'Delay Bypass' appears inactive; inserting toggle before setting 'Delay Feedback'
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: set fx 0 param Gain -> 0.550
  log: set fx 0 param Treble -> 0.700
  log: set fx 0 param Presence -> 0.650

- Swedish chainsaw (niche) [chainsaw_distortion_bypassed] => PASS (vertex-gemini)
  tone: The iconic Swedish 'chainsaw' death metal tone, popularized by bands like Entombed and Dismember. This sound is achieved…
  engineer_score: 100 (warnings: 0)
  mapping_warnings: 1
    - Section gate 'Distortion Bypass' appears inactive; inserting toggle before setting 'Drive'
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: set fx 0 param Bass -> 0.900
  log: set fx 0 param Gain -> 0.800
  log: set fx 0 param Treble -> 0.950

- Funk compressor (niche) [funk_compressor_disabled] => PASS (vertex-gemini)
  tone: A percussive, bright, and ultra-clean funk tone inspired by Nile Rodgers. Features moderate compression for a tight, 'ch…
  engineer_score: 100 (warnings: 0)
  mapping_warnings: 1
    - Section gate 'Compressor Bypass' appears inactive; inserting toggle before setting 'Makeup'
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: enabled fx 1
  log: set fx 0 param Presence -> 0.750
  log: set fx 0 param Treble -> 0.800

- Tubescreamer tighten (niche) [overdrive_bypassed] => PASS (vertex-gemini)
  tone: A tight, modern high-gain tone using a tubescreamer-style overdrive as a boost to tighten the low-end. The overdrive has…
  engineer_score: 100 (warnings: 0)
  mapping_warnings: 2
    - Section gate 'Overdrive Bypass' appears inactive; inserting toggle before setting 'Drive'
    - pruned effects: 2 -> 1
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: set fx 0 param Presence -> 0.700
  log: set fx 0 param Gain -> 0.750
  log: set fx 0 param Treble -> 0.650

- Contradiction: keep reverb OFF [bypassed_reverb] => PASS (vertex-gemini)
  tone: A versatile, foundational clean tone on the edge of breakup, suitable for a wide range of styles. The core sound is bala…
  engineer_score: 100 (warnings: 0)
  invariants:
    - enable_action_before_set: true
    - plugins_enabled_if_params_set: true
    - no_param_changes_while_inactive: true
    - delay_bypass_cleared_if_delay_set: true
    - gate_enable_cleared_if_threshold_set: true
    - reverb_bypass_cleared_if_reverb_set: true
    - eq_bypass_cleared_if_eq_set: true
  log: set fx 0 param Presence -> 0.600
  log: set fx 0 param Gain -> 0.450
  log: set fx 0 param Mid -> 0.600
```
